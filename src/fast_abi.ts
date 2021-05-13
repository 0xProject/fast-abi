import { DataItem, MethodAbi } from 'ethereum-types';
import { BigNumber } from 'bignumber.js';

const { Coder } = require('../native');

interface Opts {
    BigNumber: any;
}

export class FastABI {
    private readonly _coder: any;
    private readonly _abi: MethodAbi[];
    private readonly _opts: Opts;

    constructor(abi: MethodAbi[], opts?: Opts) {
        this._opts = { BigNumber: BigNumber, ...opts } || { BigNumber: BigNumber };
        this._abi = abi;
        this._coder = new Coder(JSON.stringify(abi));
    }

    public encodeInput(fnName: string, values: any[]): string {
        const found = this._abi.filter((a) => a.name === fnName)[0];
        const args = this._serializeArgsOut(values, found.inputs);
        try {
            const encoded = this._coder.encodeInput(fnName, args);
            return `0x${encoded}`;
        } catch (e) {
            throw new Error(`${e.message}.\nvalues=${JSON.stringify(values)}\nargs=${JSON.stringify(args)}`);
        }
    }

    public decodeInput(fnName: string, output: string): any {
        const found = this._abi.filter((a) => a.name === fnName)[0];
        const decoded = this._coder.decodeInput(fnName, output);
        return this._deserializeResultsIn(found.inputs, decoded);
    }

    public decodeOutput(fnName: string, output: string): any {
        const found = this._abi.filter((a) => a.name === fnName)[0];
        const decoded = this._coder.decodeOutput(fnName, output);
        return this._deserializeResultsIn(found.outputs, decoded);
    }

    private _deserializeResultIn(abi: DataItem, value: any): any {
        if (abi.type.indexOf('[]') !== -1) {
            // Pop off the last [] and serialize each sub value
            let type = abi.type.split('[]');
            type.pop();
            let newType = type.join('[]'); // e.g address[][] -> address[]
            return (value as any[]).map((v) => this._deserializeResultIn({ ...abi, type: newType }, v));
        }
        if (abi.type.indexOf('int') !== -1) {
            return new this._opts.BigNumber(value);
        }
        if (abi.type === 'tuple' && abi.components) {
            const output: any = {};
            for (const [i, c] of Object.entries(abi.components)) {
                output[c.name] = this._deserializeResultIn(c, value[parseInt(i)]);
            }
            return output;
        }
        return value;
    }

    private _deserializeResultsIn(abis: DataItem[], values: any[]): any {
        const output: any = [];
        for (const [i, v] of Object.entries(abis)) {
            output.push(this._deserializeResultIn(v, values[parseInt(i)]));
        }
        if (abis.length === 1) {
            return output[0];
        }
        return output;
    }

    // Convert the javascript arguments into the FastAbi preferred arguments
    private _serializeArgsOut(abis: DataItem[], args: any[]): any[] {
        return abis.map((abi, i) => this._serializeArgOut(args[i], abi));
    }

    private _serializeArgOut(abi: DataItem, arg: any): any {
        if (arg === undefined) {
            throw new Error(`Encountered undefined argument`);
        }

        if (abi.type.indexOf('[]') !== -1) {
            // Pop off the last [] and serialize each sub value
            let type = abi.type.split('[]');
            type.pop();
            let newType = type.join('[]'); // e.g address[][] -> address[]
            return (arg as any[]).map((v) => this._serializeArgOut({ ...abi, type: newType }, v));
        }

        // Convert from { b: 2, a: 1 } into a component ordered value array, [1,2]
        if (abi.type === 'tuple' && abi.components) {
            const output: any[] = [];
            for (const [_i, c] of Object.entries(abi.components)) {
                output.push(this._serializeArgOut(c, arg[c.name]));
            }
            return output;
        }

        if (this._opts.BigNumber.isBigNumber(arg)) {
            return arg.toString(10);
        }

        return arg.toString();
    }
}
