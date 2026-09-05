# Стратегия качества нативной SaaiOS

## Цель

Доказать не только корректный happy path, но и то, что телефон остаётся
управляемым при падении приложения, оболочки или нового графического стека.

## Постоянная регрессионная матрица Pixel 7

| Область | Автоматическая проверка | Проверка на телефоне |
|---|---|---|
| Boot | состав image, PID 1 contract | загрузка A, доступная USB-консоль |
| Display | буфер, clipping, BGRX conversion | корректные цвета, 1080x2400x60 |
| Input | transform и hit regions | touch, multitouch, power, volume |
| Lock | state machine, blocked dispatch | wake, unlock, отсутствие input leak |
| Wayland | protocol conformance сценарии | два независимых клиента и focus |
| Shell | navigation model snapshots | четыре раздела и recovery после crash |
| Apps | manifest/schema/lifecycle | start, switch, close, crash loop |
| Context | isolation/property tests | выбор переживает cold reboot |
| Policy | allow/deny/ask negative tests | системное подтверждение невозможно обойти |
| Storage | atomic write/recovery | данные переживают cold reboot |
| Network | bind/policy tests | API остаётся только на USB-интерфейсе |
| Audio/haptics | command bounds | stereo, volume, короткий haptic tick |
| Power | timeout state machine | screen off/wake и расход в idle |

## Quality gates

### Gate A — до сборки phone image

- workspace format/lint/tests зелёные;
- protocol и manifest fixtures совместимы;
- host integration демонстрирует требуемый сценарий;
- fault injection не оставляет бесконечный restart loop.

### Gate B — до flash

- устройство идентифицировано как `panther`;
- активный слот и разблокировка проверены;
- точный target partition и rollback image известны;
- image имеет ожидаемый размер, состав и SHA-256;
- diff не содержит credentials или userdata.

### Gate C — до объявления успеха

- устройство загрузилось без ручного восстановления;
- USB recovery path доступен;
- затронутые функции прошли smoke test;
- пользователь подтвердил свойства, которые нельзя доказать удалённо;
- выполнен cold reboot для нового постоянного состояния;
- commit, image hash и фактический binary hash связаны в документации.

## Минимальные fault scenarios для app-platform

1. Обычное приложение завершается во время касания.
2. Приложение отправляет повреждённую или неполную поверхность.
3. `saai-shell` падает на разблокированном и заблокированном экране.
4. `saai-displayd` завершается до readiness и после readiness.
5. Manifest неизвестной версии или с повторяющимся `app_id`.
6. Приложение запрашивает невыданную capability.
7. Место в каталоге данных заканчивается во время атомарной записи.

Для каждого сценария ожидаемый результат — отсутствие утечки ввода/данных,
ограниченный рестарт, запись безопасной причины и доступный fallback.

## Производительность

Метрики вводятся до оптимизации:

- время от касания до представления кадра;
- доля пропущенных кадров;
- память compositor, shell и одного приложения;
- CPU в idle и при прокрутке;
- время холодного старта приложения;
- расход батареи с включённым и выключенным экраном.

GPU-ускорение принимается только если оно улучшает измерения и не ухудшает
надёжность fallback. Ощущение плавности дополняет, но не заменяет цифры.
