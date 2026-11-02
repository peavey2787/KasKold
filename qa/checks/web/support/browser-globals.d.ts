declare function jsQR(
    data: Uint8ClampedArray,
    width: number,
    height: number,
    options?: { inversionAttempts?: string },
): { binaryData?: number[] | Uint8Array } | null;

declare module '*/pkg/companion_web.js' {
    const generated: any;
    export = generated;
}
