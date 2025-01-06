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
exports.claimPoolTokensInstructionDiscriminator = exports.ClaimPoolTokensStruct = void 0;
exports.createClaimPoolTokensInstruction = createClaimPoolTokensInstruction;
const splToken = __importStar(require("@solana/spl-token"));
const beet = __importStar(require("@metaplex-foundation/beet"));
const web3 = __importStar(require("@solana/web3.js"));
/**
 * @category Instructions
 * @category ClaimPoolTokens
 * @category generated
 */
exports.ClaimPoolTokensStruct = new beet.BeetArgsStruct([['instructionDiscriminator', beet.u8]], 'ClaimPoolTokensInstructionArgs');
exports.claimPoolTokensInstructionDiscriminator = 5;
/**
 * Creates a _ClaimPoolTokens_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @category Instructions
 * @category ClaimPoolTokens
 * @category generated
 */
function createClaimPoolTokensInstruction(accounts, programId = new web3.PublicKey('5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV')) {
    const [data] = exports.ClaimPoolTokensStruct.serialize({
        instructionDiscriminator: exports.claimPoolTokensInstructionDiscriminator,
    });
    const keys = [
        {
            pubkey: accounts.depositReceipt,
            isWritable: true,
            isSigner: false,
        },
        {
            pubkey: accounts.owner,
            isWritable: true,
            isSigner: true,
        },
        {
            pubkey: accounts.vault,
            isWritable: true,
            isSigner: false,
        },
        {
            pubkey: accounts.destination,
            isWritable: true,
            isSigner: false,
        },
        {
            pubkey: accounts.feeWallet,
            isWritable: true,
            isSigner: false,
        },
        {
            pubkey: accounts.depositAuthority,
            isWritable: false,
            isSigner: false,
        },
        {
            pubkey: accounts.poolMint,
            isWritable: false,
            isSigner: false,
        },
        {
            pubkey: accounts.tokenProgram ?? splToken.TOKEN_PROGRAM_ID,
            isWritable: false,
            isSigner: false,
        },
        {
            pubkey: accounts.systemProgram ?? web3.SystemProgram.programId,
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