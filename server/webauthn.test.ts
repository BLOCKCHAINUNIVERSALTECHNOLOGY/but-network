import { describe, expect, it } from "vitest";
import { __test } from "../shared/webauthn";

describe("webauthn — PRF secret se password wrap/unwrap", () => {
  it("wrap karke unwrap karne par wahi password milta hai", async () => {
    const fakePrf = new Uint8Array(32).fill(7);
    const wrapped = await __test.wrapWithPrfSecret(fakePrf, "my-secret-password-123");
    const back = await __test.unwrapWithPrfSecret(fakePrf, {
      credentialId: "x",
      ...wrapped,
    });
    expect(back).toBe("my-secret-password-123");
  });

  it("galat PRF secret se unwrap nahi hota", async () => {
    const fakePrf = new Uint8Array(32).fill(7);
    const otherPrf = new Uint8Array(32).fill(8);
    const wrapped = await __test.wrapWithPrfSecret(fakePrf, "my-secret-password-123");
    await expect(
      __test.unwrapWithPrfSecret(otherPrf, { credentialId: "x", ...wrapped }),
    ).rejects.toThrow();
  });
});
