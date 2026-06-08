export type Settings = {
  theme: "light" | "dark";
  notificationsEnabled: boolean;
};

export function getDefaultSettings(): Settings {
  return {
    theme: "light",
    notificationsEnabled: true,
  };
}

export function validateSettings(input: Settings): string[] {
  const errors: string[] = [];

  if (input.theme !== "light" && input.theme !== "dark") {
    errors.push("theme must be light or dark");
  }

  if (typeof input.notificationsEnabled !== "boolean") {
    errors.push("notificationsEnabled must be boolean");
  }

  return errors;
}

