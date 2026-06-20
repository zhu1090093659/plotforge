export function errorMessage(source: unknown): string {
  if (source instanceof Error) {
    return source.message;
  }

  if (typeof source === "string") {
    return source;
  }

  if (source && typeof source === "object") {
    const record = source as Record<string, unknown>;
    if (typeof record.message === "string") {
      return record.message;
    }
    if (typeof record.error === "string") {
      return record.error;
    }
    if (typeof record.code === "string") {
      const serialized = serializeObject(record);
      return serialized ? `${record.code}: ${serialized}` : record.code;
    }
    const serialized = serializeObject(record);
    return serialized ?? "Unknown error object";
  }

  return String(source);
}

function serializeObject(record: Record<string, unknown>): string | null {
  try {
    return JSON.stringify(record);
  } catch {
    return null;
  }
}
