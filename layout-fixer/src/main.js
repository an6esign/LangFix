const tauriInvoke =
  window.__TAURI__?.core?.invoke ||
  window.__TAURI_INTERNALS__?.invoke ||
  null;

async function invoke(command, args = {}) {
  if (!tauriInvoke) {
    throw new Error("Tauri API is unavailable in this window");
  }

  return tauriInvoke(command, args);
}

const tabs = Array.from(document.querySelectorAll(".tab"));
const panels = Array.from(document.querySelectorAll(".tab-panel"));

const fixBtn = document.getElementById("fixBtn");
const accessBtn = document.getElementById("accessBtn");
const doneBtn = document.getElementById("doneBtn");
const saveSettingsBtn = document.getElementById("saveSettingsBtn");
const resetBtn = document.getElementById("resetBtn");
const checkUpdatesBtn = document.getElementById("checkUpdatesBtn");
const supportBtn = document.getElementById("supportBtn");

const guideResult = document.getElementById("guideResult");
const settingsResult = document.getElementById("settingsResult");
const aboutResult = document.getElementById("aboutResult");
const aboutUpdateStatus = document.getElementById("aboutUpdateStatus");
const accessStatus = document.getElementById("accessStatus");
const welcomeBadge = document.getElementById("welcomeBadge");
const onboardingSection = document.getElementById("onboardingSection");
const heroHotkey = document.getElementById("heroHotkey");
const guideHotkey = document.getElementById("guideHotkey");

const hotkeyCaptureBtn = document.getElementById("hotkeyCaptureBtn");
const hotkeyHelp = document.getElementById("hotkeyHelp");
const autostartToggle = document.getElementById("autostartToggle");
const captureOverlay = document.getElementById("captureOverlay");
const captureDialog = document.querySelector(".captureDialog");
const capturePreview = document.getElementById("capturePreview");
const captureHint = document.getElementById("captureHint");

let currentSettings = null;
let pendingShortcut = "";
let captureMode = false;
let captureListener = null;
let updateReadyToInstall = false;

function setAboutUpdateStatus(message, tone = "") {
  aboutUpdateStatus.textContent = message;
  aboutUpdateStatus.className = "about-update-status";
  if (tone) {
    aboutUpdateStatus.classList.add(tone);
  }
}

function setResult(element, message, tone = "") {
  element.textContent = message;
  element.className = "result";
  if (tone) {
    element.classList.add(tone);
  }
}

function openTab(tabName) {
  tabs.forEach((tab) => {
    const active = tab.dataset.tab === tabName;
    tab.classList.toggle("is-active", active);
  });

  panels.forEach((panel) => {
    const active = panel.dataset.panel === tabName;
    panel.hidden = !active;
    panel.classList.toggle("is-active", active);
  });
}

function syncHotkeyLabels(label) {
  heroHotkey.textContent = label;
  guideHotkey.textContent = label;
  hotkeyCaptureBtn.textContent = label;
}

function normalizeCode(code) {
  const allowedCodes = new Set([
    "Minus",
    "Equal",
    "Comma",
    "Period",
    "Slash",
    "Backquote",
    "Semicolon",
    "Quote",
    "BracketLeft",
    "BracketRight",
    "Backslash",
    "Space",
    "Escape",
    "Enter",
    "Tab",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
  ]);

  if (/^Key[A-Z]$/.test(code) || /^Digit[0-9]$/.test(code) || allowedCodes.has(code)) {
    return code;
  }

  if (/^F([1-9]|1[0-2])$/.test(code)) {
    return code;
  }

  return null;
}

function formatShortcutLabel(shortcut) {
  return shortcut
    .split("+")
    .map((part) => {
      const upper = part.toUpperCase();
      if (["COMMAND", "CMD", "SUPER", "COMMANDORCONTROL", "COMMANDORCTRL", "CMDORCTRL", "CMDORCONTROL"].includes(upper)) {
        return "⌘";
      }
      if (["CONTROL", "CTRL"].includes(upper)) {
        return "⌃";
      }
      if (upper === "SHIFT") {
        return "⇧";
      }
      if (["ALT", "OPTION"].includes(upper)) {
        return "⌥";
      }
      if (part.startsWith("Key")) {
        return part.slice(3);
      }
      if (part.startsWith("Digit")) {
        return part.slice(5);
      }
      return (
        {
          Minus: "-",
          Equal: "=",
          Comma: ",",
          Period: ".",
          Slash: "/",
          Backquote: "`",
          Semicolon: ";",
          Quote: "'",
          BracketLeft: "[",
          BracketRight: "]",
          Backslash: "\\",
          ArrowUp: "↑",
          ArrowDown: "↓",
          ArrowLeft: "←",
          ArrowRight: "→",
          Escape: "Esc",
          Enter: "Enter",
          Tab: "Tab",
          Space: "Space",
        }[part] || part
      );
    })
    .join(" + ");
}

function startCapture() {
  stopCapture();
  captureMode = true;
  hotkeyCaptureBtn.classList.add("is-capturing");
  hotkeyCaptureBtn.textContent = "Нажмите новое сочетание…";
  hotkeyHelp.textContent = "Esc отменяет запись. Нужен хотя бы один модификатор.";
  setResult(settingsResult, "");
  captureOverlay.hidden = false;
  capturePreview.textContent = formatShortcutLabel(pendingShortcut || currentSettings?.shortcut || "Command+Shift+KeyK");
  captureHint.textContent = "Нужен хотя бы один модификатор. Esc отменяет запись.";
  captureDialog.focus();

  captureListener = (event) => {
    if (!captureMode) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      stopCapture();
      hotkeyHelp.textContent = "Запись отменена.";
      return;
    }

    const code = normalizeCode(event.code);
    if (!code) {
      captureHint.textContent = "Это сочетание не поддерживается. Попробуйте другую клавишу.";
      return;
    }

    const modifiers = [];
    if (event.metaKey) modifiers.push("Command");
    if (event.ctrlKey) modifiers.push("Control");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");

    if (modifiers.length === 0) {
      captureHint.textContent =
        "Добавьте хотя бы один модификатор: Command, Control, Alt или Shift.";
      return;
    }

    pendingShortcut = [...modifiers, code].join("+");
    capturePreview.textContent = formatShortcutLabel(pendingShortcut);
    hotkeyHelp.textContent = "Сочетание записано. Нажмите «Сохранить настройки».";
    stopCapture();
  };

  captureDialog.addEventListener("keydown", captureListener, true);
}

function stopCapture() {
  if (captureListener) {
    captureDialog.removeEventListener("keydown", captureListener, true);
    captureListener = null;
  }
  captureMode = false;
  hotkeyCaptureBtn.classList.remove("is-capturing");
  captureOverlay.hidden = true;
  if (pendingShortcut) {
    hotkeyCaptureBtn.textContent = formatShortcutLabel(pendingShortcut);
  }
}

function applySettings(settings) {
  currentSettings = settings;
  pendingShortcut = settings.shortcut;
  autostartToggle.checked = settings.launchAtLogin;
  syncHotkeyLabels(settings.shortcutLabel);
  hotkeyCaptureBtn.textContent = settings.shortcutLabel;

  if (settings.firstLaunch) {
    welcomeBadge.hidden = false;
    onboardingSection.hidden = false;
    doneBtn.hidden = false;
    openTab("guide");
  } else {
    welcomeBadge.hidden = true;
    onboardingSection.hidden = true;
    doneBtn.hidden = true;
    openTab("settings");
  }
}

async function refreshSettings() {
  const settings = await invoke("get_settings");
  applySettings(settings);
}

tabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    openTab(tab.dataset.tab);
  });
});

hotkeyCaptureBtn.addEventListener("click", (event) => {
  event.preventDefault();
  if (captureMode) {
    stopCapture();
    hotkeyHelp.textContent = "Запись отменена.";
    return;
  }
  startCapture();
});

captureOverlay.addEventListener("click", (event) => {
  if (event.target === captureOverlay) {
    stopCapture();
    hotkeyHelp.textContent = "Запись отменена.";
  }
});

fixBtn.addEventListener("click", async () => {
  fixBtn.disabled = true;
  try {
    await invoke("fix_selected_text");
    setResult(guideResult, "Проверка прошла: LangFix попытался исправить выделенный текст.", "ok");
  } catch (e) {
    setResult(guideResult, "Ошибка: " + e, "error");
    accessStatus.textContent = "Возможно, не выдан доступ в «Универсальный доступ».";
  } finally {
    fixBtn.disabled = false;
  }
});

accessBtn.addEventListener("click", async () => {
  accessBtn.disabled = true;
  try {
    await invoke("open_accessibility_settings");
    setResult(
      guideResult,
      "Настройки доступа открыты. Разрешите LangFix в списке приложений.",
      "ok",
    );
  } catch (e) {
    setResult(guideResult, "Не удалось открыть настройки: " + e, "error");
  } finally {
    accessBtn.disabled = false;
  }
});

doneBtn.addEventListener("click", async () => {
  doneBtn.disabled = true;
  try {
    await invoke("complete_onboarding");
    setResult(
      guideResult,
      "Onboarding завершён. LangFix продолжит работать через menu bar.",
      "ok",
    );
  } catch (e) {
    setResult(guideResult, "Не удалось завершить настройку: " + e, "error");
    doneBtn.disabled = false;
  }
});

saveSettingsBtn.addEventListener("click", async () => {
  saveSettingsBtn.disabled = true;
  try {
    const settings = await invoke("save_settings", {
      shortcut: pendingShortcut,
      launchAtLogin: autostartToggle.checked,
    });
    applySettings(settings);
    hotkeyHelp.textContent = "Настройки сохранены.";
    setResult(settingsResult, "Настройки успешно сохранены.", "ok");
  } catch (e) {
    setResult(settingsResult, String(e), "error");
  } finally {
    saveSettingsBtn.disabled = false;
  }
});

resetBtn.addEventListener("click", async () => {
  resetBtn.disabled = true;
  try {
    const settings = await invoke("reset_settings");
    applySettings(settings);
    hotkeyHelp.textContent = "Восстановлены стандартные параметры.";
    setResult(settingsResult, "Заводские настройки восстановлены.", "ok");
  } catch (e) {
    setResult(settingsResult, "Не удалось выполнить сброс: " + e, "error");
  } finally {
    resetBtn.disabled = false;
  }
});

supportBtn.addEventListener("click", () => {
  setResult(
    aboutResult,
    "Кнопка поддержки пока работает как заглушка. Когда будет ссылка на донаты или сайт, я подключу её сюда.",
  );
});

checkUpdatesBtn.addEventListener("click", async () => {
  checkUpdatesBtn.disabled = true;

  try {
    if (updateReadyToInstall) {
      setAboutUpdateStatus("Доступно обновление. Устанавливаю новую версию…");
      setResult(aboutResult, "Скачиваю и устанавливаю обновление…");
      await invoke("install_update");
      setResult(aboutResult, "Обновление установлено. LangFix перезапускается.", "ok");
      return;
    }

    const result = await invoke("check_for_updates");
    if (!result.configured) {
      updateReadyToInstall = false;
      checkUpdatesBtn.textContent = "Обновления недоступны";
      setAboutUpdateStatus(result.message);
      setResult(aboutResult, result.message);
      return;
    }

    if (!result.available) {
      updateReadyToInstall = false;
      checkUpdatesBtn.textContent = "Актуальная версия";
      setAboutUpdateStatus(result.message, "ok");
      setResult(aboutResult, "У вас уже установлена последняя версия.", "ok");
      return;
    }

    const update = result.update;
    updateReadyToInstall = true;
    checkUpdatesBtn.textContent = `Установить ${update.version}`;
    setAboutUpdateStatus(`Доступна новая версия ${update.version}.`, "ok");
    setResult(
      aboutResult,
      `Доступна версия ${update.version}. Нажми кнопку ещё раз, чтобы скачать и установить обновление.`,
      "ok",
    );
  } catch (e) {
    updateReadyToInstall = false;
    checkUpdatesBtn.textContent = "Проверить обновления";
    setAboutUpdateStatus("Не удалось проверить обновления.", "error");
    setResult(aboutResult, String(e), "error");
  } finally {
    checkUpdatesBtn.disabled = false;
  }
});

refreshSettings().catch((e) => {
  setResult(guideResult, "Ошибка: " + e, "error");
});
