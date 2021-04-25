import { DataItem, MethodAbi } from 'ethereum-types';
import { BigNumber } from 'bignumber.js';

var uuid = require('uuid');

var addon = require('../native');

function isObject(arg: any): arg is Object {
    return arg.constructor.name === 'Object';
}

function isString(arg: any): arg is String {
    return typeof arg === 'string' || arg instanceof String;
}

interface Opts {
    BigNumber: any;
}
export class FastABI {
    public static ping() {
        addon.hello();
    }

    private readonly _key: string;
    private readonly _abi: MethodAbi[];
    private readonly _opts: Opts;

    constructor(abi: MethodAbi[], opts?: Opts) {
        this._opts = { BigNumber: BigNumber, ...opts } || { BigNumber: BigNumber };
        this._key = uuid.v4();
        this._abi = abi;
        addon.loadAbi(this._key, JSON.stringify(abi));
    }

    public encodeInput(fnName: string, values: any[]): string {
        const found = this._abi.filter((a) => a.name === fnName);
        const args = this._convertArgs(values, found[0]);
        try {
            const encoded = addon.encodeInput(this._key, fnName, args);
            return `0x${encoded}`;
        } catch (e) {
            throw new Error(`${e.message}.\nvalues=${JSON.stringify(values)}\nargs=${JSON.stringify(args)}`);
        }
    }

    public decodeInput(fnName: string, output: string): any {
        const found = this._abi.filter((a) => a.name === fnName)[0];
        const decoded = addon.decodeInput(this._key, fnName, output);
        return this._convertDataItems(found.inputs, decoded);
    }

    public decodeOutput(fnName: string, output: string): any {
        const found = this._abi.filter((a) => a.name === fnName)[0];
        const decoded = addon.decodeOutput(this._key, fnName, output);
        return this._convertDataItems(found.outputs, decoded);
    }

    private _convertDataItem(abi: DataItem, value: any): any {
        if (abi.type.indexOf('[]') !== -1) {
            return (value as any[]).map((v) => this._convertDataItem({ ...abi, type: abi.type.split('[]')[0] }, v));
        }
        if (abi.type.indexOf('int') !== -1) {
            return new this._opts.BigNumber(value);
        }
        if (abi.type === 'tuple' && abi.components) {
            const output: any = {};
            for (const [i, c] of Object.entries(abi.components)) {
                output[c.name] = this._convertDataItem(c, value[parseInt(i)]);
            }
            return output;
        }
        return value;
    }

    private _convertDataItems(abi: DataItem[], values: any[]): any {
        const output: any = [];
        for (const [i, v] of Object.entries(abi)) {
            output.push(this._convertDataItem(v, values[parseInt(i)]));
        }
        if (abi.length === 1) {
            return output[0];
        }
        return output;
    }

    private _convertArg(arg: any): any {
        if (arg === undefined) {
            throw new Error(`Encountered undefined argument`);
        }
        // Remove any prepending 0x
        if (typeof arg === 'string' || arg instanceof String) {
            arg;
        }

        if (Array.isArray(arg)) {
            return arg.map((a) => this._convertArg(a));
        }

        if (arg.constructor.name === 'Object') {
            return Object.values(arg).map((a) => this._convertArg(a));
        }

        return arg.toString();
    }

    private _convertArgs(args: any[], abi: MethodAbi): any[] {
        return args.map((a, i) => {
            if (isObject(a)) {
                // Ensure the values are in the correct order
                const clone: any = {};
                abi.inputs[i].components!.map((k) => (clone[k.name] = a[k.name]));
                return this._convertArg(clone);
            }
            return this._convertArg(a);
        });
    }
}
