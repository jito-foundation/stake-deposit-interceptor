"use strict";
/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.updateStakePoolDepositStakeAuthorityInstructionDiscriminator = exports.UpdateStakePoolDepositStakeAuthorityStruct = void 0;
exports.createUpdateStakePoolDepositStakeAuthorityInstruction = createUpdateStakePoolDepositStakeAuthorityInstruction;
const beet = __importStar(require("@metaplex-foundation/beet"));
const web3 = __importStar(require("@solana/web3.js"));
const UpdateStakePoolDepositStakeAuthorityArgs_1 = require("../types/UpdateStakePoolDepositStakeAuthorityArgs");
/**
 * @category Instructions
 * @category UpdateStakePoolDepositStakeAuthority
 * @category generated
 */
exports.UpdateStakePoolDepositStakeAuthorityStruct = new beet.FixableBeetArgsStruct([
    ['instructionDiscriminator', beet.u8],
    [
        'updateStakePoolDepositStakeAuthorityArgs',
        UpdateStakePoolDepositStakeAuthorityArgs_1.updateStakePoolDepositStakeAuthorityArgsBeet,
    ],
], 'UpdateStakePoolDepositStakeAuthorityInstructionArgs');
exports.updateStakePoolDepositStakeAuthorityInstructionDiscriminator = 1;
/**
 * Creates a _UpdateStakePoolDepositStakeAuthority_ instruction.
 *
 * Optional accounts that are not provided default to the program ID since
 * this was indicated in the IDL from which this instruction was generated.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category UpdateStakePoolDepositStakeAuthority
 * @category generated
 */
function createUpdateStakePoolDepositStakeAuthorityInstruction(accounts, args, programId = new web3.PublicKey('5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV')) {
    const [data] = exports.UpdateStakePoolDepositStakeAuthorityStruct.serialize({
        instructionDiscriminator: exports.updateStakePoolDepositStakeAuthorityInstructionDiscriminator,
        ...args,
    });
    const keys = [
        {
            pubkey: accounts.depositStakeAuthority,
            isWritable: true,
            isSigner: false,
        },
        {
            pubkey: accounts.authority,
            isWritable: false,
            isSigner: true,
        },
        {
            pubkey: accounts.newAuthority ?? programId,
            isWritable: false,
            isSigner: false,
        },
    ];
    const ix = new web3.TransactionInstruction({
        programId,
        keys,
        data,
    });
    return ix;
}
