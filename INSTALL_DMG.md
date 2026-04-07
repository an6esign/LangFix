# Установка LangFix из DMG

## Обычная установка

1. Скачай `.dmg` из раздела Releases:

`https://github.com/an6esign/LangFix/releases`

2. Открой `.dmg`

3. Перетащи `LangFix.app` в папку `Applications`

4. Запусти приложение из `Applications`

## Если macOS блокирует запуск

Если macOS показывает сообщение, что приложение повреждено или не может быть открыто:

1. Открой Terminal
2. Выполни:

```bash
xattr -dr com.apple.quarantine /Applications/LangFix.app
```

3. Попробуй запустить LangFix снова

## После первого запуска

1. Выдай приложению доступ в `Универсальный доступ`
2. Проверь горячую клавишу
3. При желании включи автозапуск

## Важно

- лучше запускать LangFix из `Applications`, а не прямо из `Downloads`
- для автообновлений приложение тоже должно стоять в `Applications`
