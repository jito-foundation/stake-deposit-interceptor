import { PublicKey } from '@solana/web3.js'
export * from './accounts'
export * from './instructions'
export * from './types'

/**
 * Program address
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ADDRESS = '4yQFAAaf4wCKF375qihmKcHJkpkgAj8RoBxvNqt2KWf1'

/**
 * Program public key
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ID = new PublicKey(PROGRAM_ADDRESS)
