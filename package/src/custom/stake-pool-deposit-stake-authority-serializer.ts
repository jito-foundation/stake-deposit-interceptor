import { StakePoolDepositStakeAuthority, stakePoolDepositStakeAuthorityBeet } from '../generated/accounts/StakePoolDepositStakeAuthority'

export function deserialize(buf: Buffer, offset = 0): [StakePoolDepositStakeAuthority, number] {
  return stakePoolDepositStakeAuthorityBeet.deserialize(buf, offset + 8)
}

export function serialize(instance: StakePoolDepositStakeAuthority): [Buffer, number] {
  return stakePoolDepositStakeAuthorityBeet.serialize(instance)
}