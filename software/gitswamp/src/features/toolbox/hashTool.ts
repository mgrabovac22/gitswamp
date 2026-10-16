const MD5_SHIFTS = [
  7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
  5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
  4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
  6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_K = (() => {
  const table = new Uint32Array(64);
  for (let i = 0; i < 64; i += 1) {
    table[i] = Math.floor(Math.abs(Math.sin(i + 1)) * 4294967296);
  }
  return table;
})();

export function md5(input: string): string {
  const bytes = new TextEncoder().encode(input);
  const originalLength = bytes.length;
  const paddedLength = (((originalLength + 8) >> 6) + 1) << 6;
  const buffer = new Uint8Array(paddedLength);
  buffer.set(bytes);
  buffer[originalLength] = 0x80;

  const view = new DataView(buffer.buffer);
  const bitLengthHigh = Math.floor(originalLength / 536870912);
  const bitLengthLow = (originalLength << 3) >>> 0;
  view.setUint32(paddedLength - 8, bitLengthLow, true);
  view.setUint32(paddedLength - 4, bitLengthHigh, true);

  let a0 = 0x67452301;
  let b0 = 0xefcdab89;
  let c0 = 0x98badcfe;
  let d0 = 0x10325476;

  const message = new Uint32Array(16);
  for (let offset = 0; offset < paddedLength; offset += 64) {
    for (let j = 0; j < 16; j += 1) message[j] = view.getUint32(offset + j * 4, true);
    let a = a0;
    let b = b0;
    let c = c0;
    let d = d0;
    for (let i = 0; i < 64; i += 1) {
      let f: number;
      let g: number;
      if (i < 16) {
        f = (b & c) | (~b & d);
        g = i;
      } else if (i < 32) {
        f = (d & b) | (~d & c);
        g = (5 * i + 1) % 16;
      } else if (i < 48) {
        f = b ^ c ^ d;
        g = (3 * i + 5) % 16;
      } else {
        f = c ^ (b | ~d);
        g = (7 * i) % 16;
      }
      f = (f + a + MD5_K[i] + message[g]) >>> 0;
      a = d;
      d = c;
      c = b;
      const shift = MD5_SHIFTS[i];
      b = (b + ((f << shift) | (f >>> (32 - shift)))) >>> 0;
    }
    a0 = (a0 + a) >>> 0;
    b0 = (b0 + b) >>> 0;
    c0 = (c0 + c) >>> 0;
    d0 = (d0 + d) >>> 0;
  }

  const toHex = (value: number) => {
    let out = "";
    for (let i = 0; i < 4; i += 1) {
      out += ((value >>> (i * 8)) & 0xff).toString(16).padStart(2, "0");
    }
    return out;
  };

  return toHex(a0) + toHex(b0) + toHex(c0) + toHex(d0);
}

export type HashAlgorithm = "SHA-1" | "SHA-256" | "SHA-384" | "SHA-512";

export const SHA_ALGORITHMS: HashAlgorithm[] = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];

export async function shaHash(algorithm: HashAlgorithm, input: string): Promise<{ hex: string; error: string | null }> {
  try {
    const subtle = globalThis.crypto?.subtle;
    if (!subtle) {
      return { hex: "", error: "Secure hashing is unavailable in this environment." };
    }
    const data = new TextEncoder().encode(input);
    const digest = await subtle.digest(algorithm, data);
    const hex = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
    return { hex, error: null };
  } catch (err) {
    return { hex: "", error: err instanceof Error ? err.message : String(err) };
  }
}

export async function hashAll(input: string): Promise<{ algorithm: string; hex: string; error: string | null }[]> {
  const results: { algorithm: string; hex: string; error: string | null }[] = [
    { algorithm: "MD5", hex: input ? md5(input) : "", error: null },
  ];
  for (const algorithm of SHA_ALGORITHMS) {
    const result = await shaHash(algorithm, input);
    results.push({ algorithm, hex: result.hex, error: result.error });
  }
  return results;
}
