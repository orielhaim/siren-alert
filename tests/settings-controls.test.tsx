import { afterEach, beforeEach, expect, mock, test } from "bun:test";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { useState } from "react";
const calls: { command: string; args: unknown }[] = [];
mock.module("@tauri-apps/api/core", () => ({
  invoke: async (command: string, args: unknown) => {
    calls.push({ command, args });
  },
}));
mock.module("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
const { SoundPicker } = await import("../src/components/SoundPicker");
const {
  MultiSelect,
  MultiSelectTrigger,
  MultiSelectValue,
  MultiSelectInput,
  MultiSelectContent,
  MultiSelectList,
  MultiSelectItem,
} = await import("../src/components/ui/multi-select");
const { AppleSwitch } = await import("../src/components/ui/switch");
const { default: i18n } = await import("../src/i18n");
afterEach(cleanup);
beforeEach(async () => {
  calls.splice(0);
  await i18n.changeLanguage("en");
});
test("an option preview neither selects nor closes the sound combobox", async () => {
  const committed: string[] = [];
  const ui = render(
    <SoundPicker
      value="siren1"
      sounds={[
        { id: "siren1", name: "Siren 1", custom: false },
        { id: "siren2", name: "Siren 2", custom: false },
      ]}
      label="Emergency alert"
      disabled={false}
      onConfirm={(id) => committed.push(id)}
      onError={(error) => {
        throw new Error(error);
      }}
    />,
  );
  const input = ui.getByRole("combobox");
  fireEvent.focus(input);
  await waitFor(() => expect(input.getAttribute("aria-expanded")).toBe("true"));
  const preview = await ui.findByRole("button", {
    name: "Preview sound Siren 2",
  });
  fireEvent.pointerDown(preview);
  fireEvent.click(preview);
  await waitFor(() =>
    expect(calls).toContainEqual({
      command: "preview_sound",
      args: { id: "siren2" },
    }),
  );
  expect(input.getAttribute("aria-expanded")).toBe("true");
  expect(committed).toEqual([]);
  expect(
    ui.getByRole("option", { name: /Siren 1/ }).getAttribute("aria-selected"),
  ).toBe("true");
  expect(
    ui.getByRole("option", { name: /Siren 2/ }).getAttribute("aria-selected"),
  ).toBe("false");
  fireEvent.click(ui.getByRole("option", { name: /Siren 2/ }));
  await waitFor(() =>
    expect(ui.getByRole("button", { name: "Use this sound" })).toBeDefined(),
  );
  expect(committed).toEqual([]);
  fireEvent.click(ui.getByRole("button", { name: "Use this sound" }));
  expect(committed).toEqual(["siren2"]);
});
test("city multi-select preserves canonical city values while searching translations", async () => {
  const changes: string[][] = [];
  function Cities() {
    const [value, setValue] = useState<string[]>([]);
    return (
      <MultiSelect
        value={value}
        onValueChange={(next) => {
          setValue(next);
          changes.push(next);
        }}
      >
        <MultiSelectTrigger>
          <MultiSelectValue />
          <MultiSelectInput aria-label="Cities" />
        </MultiSelectTrigger>
        <MultiSelectContent>
          <MultiSelectList>
            <MultiSelectItem value="חיפה" keywords={["Haifa"]}>
              Haifa
            </MultiSelectItem>
            <MultiSelectItem value="ירושלים" keywords={["Jerusalem"]}>
              Jerusalem
            </MultiSelectItem>
          </MultiSelectList>
        </MultiSelectContent>
      </MultiSelect>
    );
  }
  const ui = render(<Cities />);
  const input = ui.getByRole("combobox");
  fireEvent.focus(input);
  fireEvent.change(input, { target: { value: "Haifa" } });
  fireEvent.click(await ui.findByRole("option", { name: "Haifa" }));
  expect(changes).toEqual([["חיפה"]]);
  expect(input.getAttribute("aria-expanded")).toBe("true");
  fireEvent.click(ui.getByRole("button", { name: "Remove Haifa" }));
  expect(changes).toEqual([["חיפה"], []]);
});
test("the supplied switch uses its controlled change callback", () => {
  const changes: boolean[] = [];
  const ui = render(
    <AppleSwitch
      checked={false}
      aria-label="Countdown"
      onCheckedChange={(value) => changes.push(value)}
    />,
  );
  fireEvent.click(ui.getByRole("switch", { name: "Countdown" }));
  expect(changes).toEqual([true]);
});
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "../src/components/ui/select";
test("the supplied select changes the language through its real selection API", async () => {
  const changes: string[] = [];
  function Language() {
    const [value, setValue] = useState("en");
    return (
      <Select
        value={value}
        onValueChange={(next) => {
          setValue(next);
          changes.push(next);
        }}
      >
        <SelectTrigger aria-label="Language">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="en">English</SelectItem>
          <SelectItem value="he">עברית</SelectItem>
        </SelectContent>
      </Select>
    );
  }
  const ui = render(<Language />);
  fireEvent.click(ui.getByRole("button", { name: "Language" }));
  fireEvent.click(await ui.findByRole("option", { name: "עברית" }));
  expect(changes).toEqual(["he"]);
});
import { act } from "@testing-library/react";
import { useTranslation } from "react-i18next";
test("React controls update immediately when i18next changes language", async () => {
  function Localized() {
    const { t } = useTranslation();
    return (
      <SoundPicker
        value="siren1"
        sounds={[{ id: "siren1", name: "Siren 1", custom: false }]}
        label={t("alert")}
        disabled={false}
        onConfirm={() => {}}
        onError={(message) => {
          throw new Error(message);
        }}
      />
    );
  }
  const ui = render(<Localized />);
  expect(ui.getByRole("combobox", { name: "Emergency alert" })).toBeDefined();
  await act(() => i18n.changeLanguage("he"));
  expect(ui.getByRole("combobox", { name: "התרעת חירום" })).toBeDefined();
  expect(ui.getByRole("button", { name: "השמעת ניסיון" })).toBeDefined();
});
