"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.deserialize = deserialize;
exports.serialize = serialize;
const DepositReceipt_1 = require("../generated/accounts/DepositReceipt");
function deserialize(buf, offset = 0) {
    return DepositReceipt_1.depositReceiptBeet.deserialize(buf, offset + 8);
}
function serialize(instance) {
    return DepositReceipt_1.depositReceiptBeet.serialize(instance);
}
