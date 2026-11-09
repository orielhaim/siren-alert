import { describe, expect, test } from "bun:test";
import { settingsWriter } from "../src/lib/settings-writer";
describe("automatic settings persistence", () => {
  test("serializes changes while a previous write is pending", async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const writes: string[] = [];
    const writer = settingsWriter<string>(async (value) => {
      writes.push(value);
      if (value === "left") await gate;
    });
    const first = writer.write("left");
    const second = writer.write("right");
    await Promise.resolve();
    expect(writes).toEqual(["left"]);
    release();
    await Promise.all([first, second]);
    expect(writes).toEqual(["left", "right"]);
  });
  test("a failed write does not discard the next change", async () => {
    const writes: string[] = [];
    const writer = settingsWriter<string>(async (value) => {
      if (value === "invalid") throw new Error("disk unavailable");
      writes.push(value);
    });
    await expect(writer.write("invalid")).rejects.toThrow("disk unavailable");
    await writer.write("latest");
    expect(writes).toEqual(["latest"]);
  });
});
