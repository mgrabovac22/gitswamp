import { fileExtension, MAX_IMAGE_PREVIEW_BYTES } from "./imagePreview";

export const MAX_IMAGE_PREVIEW_PIXELS = 8_000_000;
export const MAX_IMAGE_PREVIEW_DIMENSION = 8192;

let previewQueue: Promise<unknown> = Promise.resolve();

// A stale selection must not start another IPC read or image decode.
export function withBinaryPreviewSlot<T>(current: () => boolean, task: () => Promise<T>): Promise<T | undefined> {
  const next = previewQueue.then(() => current() ? task() : undefined);
  previewQueue = next.catch(() => undefined);
  return next;
}

function validateDimensions(width: number, height: number) {
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    throw new Error("Image dimensions could not be read safely");
  }
  if (width > MAX_IMAGE_PREVIEW_DIMENSION || height > MAX_IMAGE_PREVIEW_DIMENSION || width * height > MAX_IMAGE_PREVIEW_PIXELS) {
    throw new Error("Image exceeds the preview limit (8 megapixels or 8192 pixels per side)");
  }
}

// Inspect compressed headers before handing a file to the browser decoder.
export function validateImagePreview(bytes: Uint8Array, path: string): void {
  if (!bytes.length) throw new Error("Image is empty");
  if (bytes.length > MAX_IMAGE_PREVIEW_BYTES) throw new Error("File is too large to preview (10 MB limit)");
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const ascii = (offset: number, length: number) => String.fromCharCode(...bytes.subarray(offset, offset + length));
  let width = 0;
  let height = 0;
  try {
    if (ascii(0, 8) === "\x89PNG\r\n\x1a\n") {
      width = view.getUint32(16); height = view.getUint32(20);
      for (let offset = 8; offset + 12 <= bytes.length;) {
        const length = view.getUint32(offset);
        if (ascii(offset + 4, 4) === "acTL") throw new Error("Animated images are not previewed to limit memory use");
        offset += length + 12;
      }
    } else if (bytes[0] === 0xff && bytes[1] === 0xd8) {
      for (let offset = 2; offset + 4 <= bytes.length;) {
        if (bytes[offset++] !== 0xff) break;
        while (bytes[offset] === 0xff) offset++;
        const marker = bytes[offset++];
        if (marker === 0xda || marker === 0xd9) break;
        if (marker === 0x01 || (marker !== undefined && marker >= 0xd0 && marker <= 0xd7)) continue;
        const length = view.getUint16(offset);
        if (length < 2) break;
        if (marker !== undefined && [0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf].includes(marker)) {
          height = view.getUint16(offset + 3); width = view.getUint16(offset + 5); break;
        }
        offset += length;
      }
    } else if (["GIF87a", "GIF89a"].includes(ascii(0, 6))) {
      width = view.getUint16(6, true); height = view.getUint16(8, true);
      let frames = 0;
      let offset = 13 + ((bytes[10]! & 0x80) ? 3 * (1 << ((bytes[10]! & 7) + 1)) : 0);
      while (offset < bytes.length) {
        const block = bytes[offset++];
        if (block === 0x3b) break;
        if (block === 0x2c) {
          if (++frames > 1) throw new Error("Animated images are not previewed to limit memory use");
          validateDimensions(view.getUint16(offset + 4, true), view.getUint16(offset + 6, true));
          const packed = bytes[offset + 8]!;
          offset += 9 + ((packed & 0x80) ? 3 * (1 << ((packed & 7) + 1)) : 0);
          offset++; // LZW code size
        } else if (block === 0x21) offset++; // extension label
        else throw new Error("Invalid GIF image");
        while (offset < bytes.length) {
          const length = bytes[offset++]!;
          if (!length) break;
          offset += length;
        }
      }
    } else if (ascii(0, 4) === "RIFF" && ascii(8, 4) === "WEBP") {
      for (let offset = 12; offset + 8 <= bytes.length;) {
        const kind = ascii(offset, 4);
        const length = view.getUint32(offset + 4, true);
        const data = offset + 8;
        if (kind === "ANIM" || kind === "ANMF" || (kind === "VP8X" && (bytes[data]! & 2))) {
          throw new Error("Animated images are not previewed to limit memory use");
        }
        if (kind === "VP8X") {
          width = 1 + (view.getUint32(data + 4, true) & 0xffffff);
          height = 1 + (bytes[data + 7]! | bytes[data + 8]! << 8 | bytes[data + 9]! << 16);
        } else if (kind === "VP8 " && !width) {
          width = view.getUint16(data + 6, true) & 0x3fff; height = view.getUint16(data + 8, true) & 0x3fff;
        } else if (kind === "VP8L" && !width) {
          const bits = view.getUint32(data + 1, true);
          width = 1 + (bits & 0x3fff); height = 1 + ((bits >>> 14) & 0x3fff);
        }
        offset += 8 + length + (length & 1);
      }
    } else if (ascii(0, 2) === "BM") {
      if (view.getUint32(14, true) === 12) {
        width = view.getUint16(18, true); height = view.getUint16(20, true);
      } else {
        width = view.getInt32(18, true); height = Math.abs(view.getInt32(22, true));
      }
    } else if (view.byteLength >= 6 && view.getUint32(0, true) === 0x10000) {
      const count = view.getUint16(4, true);
      if (count > 64) throw new Error("Icon contains too many images to preview");
      for (let i = 0; i < count; i++) {
        const entry = 6 + i * 16;
        const iconWidth = bytes[entry] || 256, iconHeight = bytes[entry + 1] || 256;
        const length = view.getUint32(entry + 8, true), offset = view.getUint32(entry + 12, true);
        if (offset + length > bytes.length) throw new Error("Icon data is incomplete");
        if (ascii(offset, 8) === "\x89PNG\r\n\x1a\n") {
          if (view.getUint32(offset + 16) !== iconWidth || view.getUint32(offset + 20) !== iconHeight) throw new Error("Icon dimensions do not match its image data");
          validateImagePreview(bytes.subarray(offset, offset + length), "icon.png");
        } else {
          const headerSize = view.getUint32(offset, true);
          const bitmapWidth = headerSize === 12 ? view.getUint16(offset + 4, true) : view.getInt32(offset + 4, true);
          const bitmapHeight = headerSize === 12 ? view.getUint16(offset + 6, true) : Math.abs(view.getInt32(offset + 8, true));
          if (bitmapWidth !== iconWidth || bitmapHeight !== iconHeight * 2) throw new Error("Icon dimensions do not match its image data");
        }
        width = Math.max(width, iconWidth); height = Math.max(height, iconHeight);
      }
    } else if (fileExtension(path) === "svg") {
      if (bytes.length > 256 * 1024) throw new Error("SVG exceeds the preview limit (256 KB)");
      const text = new TextDecoder().decode(bytes);
      if (/<!DOCTYPE|<!ENTITY/i.test(text)) throw new Error("SVG document declarations are not previewed");
      const xml = new DOMParser().parseFromString(text, "image/svg+xml");
      const svg = xml.documentElement;
      // No embedded media, recursion, filters, scripts or external references.
      if (svg.localName !== "svg" || xml.querySelector("parsererror, script, foreignObject, image, use, filter, animate, animateTransform, animateMotion, set, style") || xml.querySelectorAll("*").length > 2000) {
        throw new Error("Complex or animated SVG is not previewed to limit memory use");
      }
      for (const element of xml.querySelectorAll("*")) {
        for (const attribute of element.attributes) {
          if (/^on|href$/i.test(attribute.name) || /url\s*\(/i.test(attribute.value) || attribute.name === "style") {
            throw new Error("SVG with external or styled content is not previewed");
          }
        }
      }
      const box = (svg.getAttribute("viewBox") ?? "").trim().split(/[\s,]+/).map(Number);
      const dimension = (name: string, fallback: number) => {
        const value = svg.getAttribute(name);
        return value && /^\d+(?:\.\d+)?(?:px)?$/.test(value) ? parseFloat(value) : fallback;
      };
      width = dimension("width", box[2] ?? 300); height = dimension("height", box[3] ?? 150);
    } else {
      throw new Error("Preview is not available for this image format");
    }
    validateDimensions(width, height);
  } catch (error) {
    if (error instanceof RangeError) throw new Error("Image header is incomplete or invalid");
    throw error;
  }
}
