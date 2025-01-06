"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.deserialize = deserialize;
exports.serialize = serialize;
const StakePoolDepositStakeAuthority_1 = require("../generated/accounts/StakePoolDepositStakeAuthority");
function deserialize(buf, offset = 0) {
    return StakePoolDepositStakeAuthority_1.stakePoolDepositStakeAuthorityBeet.deserialize(buf, offset + 8);
}
function serialize(instance) {
    return StakePoolDepositStakeAuthority_1.stakePoolDepositStakeAuthorityBeet.serialize(instance);
}
