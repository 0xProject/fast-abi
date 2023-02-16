import * as ethers from 'ethers';

import BalanceCheckerAbi from './BalanceChecker.abi.json';
import { BigNumber } from 'bignumber.js';
import { FastABI } from '..';
import { MethodAbi } from 'ethereum-types';
import util from 'util';

/**
 * Tests FastABI by comparing the results of encoding/decoding with ethers.js.
 *
 * Test ABI is from https://github.com/wbobeirne/eth-balance-checker/blob/master/abis/BalanceChecker.abi.json
 */
describe('fastAbi', () => {
    it('encodes inputs', () => {
        const balanceCheckerEthersInterface = new ethers.Interface(BalanceCheckerAbi as MethodAbi[]);
        const encodedFunctionData = balanceCheckerEthersInterface.encodeFunctionData('tokenBalance', [
            '0x4Ea754349AcE5303c82f0d1D491041e042f2ad22',
            '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
        ]);

        const balanceCheckerFastAbi = new FastABI(BalanceCheckerAbi as MethodAbi[]);
        const encodedFunctionDataFast = balanceCheckerFastAbi.encodeInput('tokenBalance', [
            '0x4Ea754349AcE5303c82f0d1D491041e042f2ad22',
            '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
        ]);

        expect(encodedFunctionDataFast).toEqual(encodedFunctionData);
    });

    it('decodes inputs', () => {
        // Caldata from 'encodes inputs' test
        const calldata =
            '0x1049334f0000000000000000000000008ba1f109551bd432803012645ac136ddd64dba72000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2';
        const balanceCheckerEthersInterface = new ethers.Interface(BalanceCheckerAbi as MethodAbi[]);
        const decodedFunctionDataEthers = balanceCheckerEthersInterface.decodeFunctionData('tokenBalance', calldata);

        const balanceCheckerFastAbi = new FastABI(BalanceCheckerAbi as MethodAbi[]);
        const decodedFunctionDataFast: string[] = balanceCheckerFastAbi.decodeInput('tokenBalance', calldata);

        expect(decodedFunctionDataFast).toEqual(decodedFunctionDataEthers.map((a) => a.toLowerCase()));
    });

    it('decodes outputs', () => {
        const result = '0x00000000000000000000000000000000000000000000000000eb01cd45901fac';
        const balanceCheckerEthersInterface = new ethers.Interface(BalanceCheckerAbi as MethodAbi[]);
        const decodedOutputEthers: bigint[] = balanceCheckerEthersInterface.decodeFunctionResult(
            'tokenBalance',
            result,
        );

        const balanceCheckerFastAbi = new FastABI(BalanceCheckerAbi as MethodAbi[]);
        const decodedOutputFast: BigNumber = balanceCheckerFastAbi.decodeOutput('tokenBalance', result);

        expect(decodedOutputFast.toNumber()).toEqual(Number(decodedOutputEthers[0]));
    });
});
