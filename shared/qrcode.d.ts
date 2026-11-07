/**
 * Minimal type declarations for the `qrcode` package (v1.5.x), which
 * ships no TypeScript types of its own. Only the functions the wallet
 * uses are declared here.
 */
declare module "qrcode" {
  export interface QRCodeToDataURLOptions {
    errorCorrectionLevel?: "L" | "M" | "Q" | "H";
    margin?: number;
    width?: number;
    color?: { dark?: string; light?: string };
  }

  export function toDataURL(
    text: string,
    options?: QRCodeToDataURLOptions,
  ): Promise<string>;

  export interface QRCodeModules {
    size: number;
    data: Uint8Array;
  }

  export interface QRCodeObject {
    modules: QRCodeModules;
    version: number;
  }

  export function create(
    text: string,
    options?: { errorCorrectionLevel?: "L" | "M" | "Q" | "H" },
  ): QRCodeObject;
}
