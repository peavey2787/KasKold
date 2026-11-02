// The vendored jsQR script installs this browser global before any module runs.
declare function jsQR(
  data: Uint8ClampedArray,
  width: number,
  height: number,
  options?: { inversionAttempts?: string },
): { data: string; binaryData: number[] } | null;
