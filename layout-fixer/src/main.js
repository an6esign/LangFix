import { invoke } from "@tauri-apps/api/core";

const btn = document.getElementById("fixBtn");
const result = document.getElementById("result");

btn.addEventListener("click", async () => {
  try {
    await invoke("fix_selected_text");
    result.textContent = "Попытка исправить выделенный текст выполнена";
  } catch (e) {
    result.textContent = "Ошибка: " + e;
  }
});
