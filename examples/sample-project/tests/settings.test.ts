import { getDefaultSettings, validateSettings } from "../src/settings";

test("default settings are valid", () => {
  expect(validateSettings(getDefaultSettings())).toEqual([]);
});

test("invalid theme is rejected", () => {
  expect(
    validateSettings({
      theme: "" as "light",
      notificationsEnabled: true,
    }),
  ).toContain("theme must be light or dark");
});

