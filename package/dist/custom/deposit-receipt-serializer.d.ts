import { DepositReceipt } from '../generated/accounts/DepositReceipt';
export declare function deserialize(buf: Buffer, offset?: number): [DepositReceipt, number];
export declare function serialize(instance: DepositReceipt): [Buffer, number];
