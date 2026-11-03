const IMAGE_EXTENSIONS = new Set([
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "bmp",
  "ico",
  "avif",
  "svg",
  "tif",
  "tiff",
])

const MIME_TYPES: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  bmp: "image/bmp",
  ico: "image/x-icon",
  avif: "image/avif",
  svg: "image/svg+xml",
  tif: "image/tiff",
  tiff: "image/tiff",
}

export const MAX_IMAGE_PREVIEW_BYTES = 10 * 1024 * 1024

export function fileExtension(filePath: string): string {
  const withoutQuery = filePath.split(/[?#]/, 1)[0] ?? filePath
  const lastSegment = withoutQuery.split(/[\\/]/).pop() ?? withoutQuery
  const dotIndex = lastSegment.lastIndexOf(".")
  return dotIndex >= 0 ? lastSegment.slice(dotIndex + 1).toLowerCase() : ""
}

export function isImageFilePath(filePath: string): boolean {
  return IMAGE_EXTENSIONS.has(fileExtension(filePath))
}

export function imageMimeType(filePath: string): string {
  return MIME_TYPES[fileExtension(filePath)] ?? "application/octet-stream"
}
