import { DepositReceipt, depositReceiptBeet } from '../generated/accounts/DepositReceipt'

export function deserialize(buf: Buffer, offset = 0): [DepositReceipt, number] {
  return depositReceiptBeet.deserialize(buf, offset + 8)
}

export function serialize(instance: DepositReceipt): [Buffer, number] {
  return depositReceiptBeet.serialize(instance)
}