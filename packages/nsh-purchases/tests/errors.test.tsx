import { expect, test } from "nsh/test";
import {
  asPurchasesError,
  purchasesError,
  PurchasesError,
  type PurchaseErrorCode,
} from "../src";

const CODES: PurchaseErrorCode[] = [
  "ERR_PURCHASES_NOT_AVAILABLE",
  "ERR_PURCHASES_NOT_INITIALIZED",
  "ERR_PURCHASES_IN_PROGRESS",
  "ERR_PURCHASES_TIMEOUT",
  "ERR_PURCHASES_CANCELLED",
  "ERR_PURCHASES_PRODUCT_NOT_FOUND",
  "ERR_PURCHASES_ALREADY_OWNED",
  "ERR_PURCHASES_NOT_OWNED",
  "ERR_PURCHASES_PAYMENT_INVALID",
  "ERR_PURCHASES_NOT_ALLOWED",
  "ERR_PURCHASES_NETWORK",
  "ERR_PURCHASES_INVALID_ARGUMENT",
  "ERR_PURCHASES_BACKEND",
];

/** The crate's wording, from crates/nsh-purchases/src/error.rs, message by message. */
const RUST_MESSAGES: Record<PurchaseErrorCode, string> = {
  ERR_PURCHASES_NOT_AVAILABLE:
    "In-app purchases are not available on this target, or no host has registered.",
  ERR_PURCHASES_NOT_INITIALIZED:
    "Call initialize() and wait for it to succeed before using the store.",
  ERR_PURCHASES_IN_PROGRESS: "Another purchase is already running. Wait for it to finish.",
  ERR_PURCHASES_TIMEOUT: "The purchase timed out waiting for its answer. Try again.",
  ERR_PURCHASES_CANCELLED: "The purchase was cancelled.",
  ERR_PURCHASES_PRODUCT_NOT_FOUND: "The store has no product with that id.",
  ERR_PURCHASES_ALREADY_OWNED: "The product is already owned.",
  ERR_PURCHASES_NOT_OWNED: "The purchase is unknown or already finished.",
  ERR_PURCHASES_PAYMENT_INVALID: "The store rejected the payment.",
  ERR_PURCHASES_NOT_ALLOWED: "The purchase is not allowed on this device or account.",
  ERR_PURCHASES_NETWORK: "The store connection failed. Check the network and try again.",
  ERR_PURCHASES_INVALID_ARGUMENT:
    "An argument is empty or not allowed, such as a zero quantity or a non-consumable consume.",
  ERR_PURCHASES_BACKEND: "The platform's store service failed.",
};

test("known codes pass through with their own code", () => {
  for (const code of CODES) {
    expect(asPurchasesError(purchasesError(code)).code).toBe(code);
    expect(asPurchasesError({ code, message: "host text" }).code).toBe(code);
  }
});

test("a host rejection is parsed from the code its message starts with", () => {
  const parsed = asPurchasesError({
    code: "ERR_NSH_HOST_ERROR",
    message: "ERR_PURCHASES_CANCELLED: the sheet was dismissed",
  });
  expect(parsed.code).toBe("ERR_PURCHASES_CANCELLED");
  // The host's raw text is dropped: the parsed error carries the crate's wording.
  expect(parsed.message).toBe(RUST_MESSAGES["ERR_PURCHASES_CANCELLED"]);
});

test("an unknown prefix, an unknown code and a bare throw become backend failures", () => {
  expect(
    asPurchasesError({ code: "ERR_NSH_HOST_ERROR", message: "ERR_SOMETHING_ELSE: nope" }).code,
  ).toBe("ERR_PURCHASES_BACKEND");
  expect(asPurchasesError({ code: "ERR_PURCHASES_NOT_A_CODE", message: "x" }).code).toBe(
    "ERR_PURCHASES_BACKEND",
  );
  expect(asPurchasesError(new Error("boom")).code).toBe("ERR_PURCHASES_BACKEND");
  expect(asPurchasesError(null).code).toBe("ERR_PURCHASES_BACKEND");
  expect(asPurchasesError(undefined).code).toBe("ERR_PURCHASES_BACKEND");
});

test("the messages match the Rust crate's wording", () => {
  for (const code of CODES) {
    const error = purchasesError(code);
    expect(error.message).toBe(RUST_MESSAGES[code]);
    expect(new PurchasesError(code).message).toBe(RUST_MESSAGES[code]);
    expect(error.name).toBe("PurchasesError");
  }
});

test("every code in the union has its message and back", () => {
  // One message per code, no code worded twice: the union and the record stay aligned.
  const messages = CODES.map((code) => RUST_MESSAGES[code]);
  expect(new Set(messages).size).toBe(CODES.length);
  expect(CODES.length).toBe(13);
});
