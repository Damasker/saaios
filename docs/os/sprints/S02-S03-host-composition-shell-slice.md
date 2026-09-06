# Подготовительный host-срез S02/S03

## Статус и границы

- Состояние среза: `Implemented`.
- S02 и S03 остаются `Backlog`; их device acceptance не выполнен.
- Срез работает только на headless host и не изменяет phone image.
- DRM/KMS, Pixel 7, GPU, `linux-dmabuf` protocol global и аппаратная
  композиция не реализованы и не заявляются.

## Результат

`saai-displayd` получил тестируемую границу composition backend.
`wl_shm` доступен безусловно. `dmabuf` считается доступным только одновременно
при наличии runtime capability и успешной проверке импорта конкретного
descriptor. Отсутствие capability, descriptor или отказ импорта выбирают
`wl_shm`; отказ не завершает compositor.

Отдельный workspace crate и процесс `saai-shell` подключается через настоящий
Wayland socket, проходит `xdg_surface` configure/ack, создаёт XRGB8888
`wl_shm` buffer и рисует фиксированную shell-поверхность. Это только
детерминированный host-прототип компоновки header/card/navigation, а не
реализация lock screen или мобильной оболочки S03.

## Проверки

- unit: `wl_shm` всегда доступен;
- unit: без backend capability dmabuf import check не вызывается;
- unit: отказ import check возвращает план `wl_shm` без ошибки процесса;
- unit: dmabuf доступен только после capability и успешного import check;
- multi-process: compositor принимает shell через реальный Wayland protocol;
- determinism: shell frame hash равен
  `2bba4517e6540d3af0ec30fbcaf5818733d7e685807608f4189381069576155a`;
- recovery: после нормального выхода первого shell compositor остаётся жив,
  принимает второй процесс shell и получает тот же hash.

## Что ещё требуется

S02 по-прежнему требует отдельного проверенного DRM/KMS backend на устройстве,
цветовой проверки, evdev touch, readiness/watchdog, cold boot и rollback.
S03 по-прежнему требует системных Wayland-протоколов, lock/input isolation,
полной оболочки, restart policy с лимитом и device UI/touch regression.
Решение о dmabuf/GPU откладывается до runtime backend с реальным importer и
измеримыми device evidence.
