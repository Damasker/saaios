# ADR-045: S12 Change 2 -- схема подписи и versioned manifest для OTA

## Статус

Принято, 2026-09-12.

## Контекст

S12's Definition of Ready (`docs/os/sprints/S12-ota-release-gate.md`)
поставил Change 2 второй задачей, полностью host-testable: схема
подписи manifest+artifact и формат versioned manifest с
anti-downgrade. До этого спринта в проекте не было ни одной
крипто-подписи (`grep` на `ed25519`/`dalek`/`ring =`/`rsa =`/
`Signature` по всем `Cargo.toml` -- ноль совпадений, подтверждено
дважды: при написании S11's DoR и S12's DoR).

## Решение

Новый крейт `crates/saai-ota-manifest`:

- **Схема подписи -- ed25519** (`ed25519-dalek = "2"`). Выбран как
  лёгкий, чистый Rust, без OpenSSL/vendor-зависимостей на устройстве --
  соответствует S12 DoR's Threat/privacy impact ("без OpenSSL на
  устройстве"). Приватный `SigningKey` никогда не сериализуется в
  манифест и не встраивается в образ -- только `VerifyingKey`
  (публичный) передаётся проверяющей стороне.
- **`ManifestBody`** (`#[serde(deny_unknown_fields)]`, тот же паттерн
  строгой схемы, что `saai-app-protocol`'s `ClientRequest`): `schema`
  (u32, сейчас `OTA_MANIFEST_SCHEMA_V1 = 1`), `target` (должен совпадать
  с реальным `androidboot.hardware` устройства -- `system_identity()`'s
  `target`, сегодня `panther`), `version`/`min_supported_version`
  (semver, `semver` крейт уже был workspace-зависимостью), `partitions`
  (`Vec<PartitionEntry { name, sha256, size_bytes }>`), `built_at`
  (RFC3339, информационно), `notes` (опционально).
- **Канонизация подписи** -- никакой отдельной схемы канонизации не
  реализовано: подписывающая и проверяющая стороны используют один и
  тот же Rust-тип `ManifestBody` (все поля -- `String`/`u64`/`Vec`/
  `Option`, ни одного `HashMap`), поэтому `serde_json::to_vec` уже
  детерминирован по построению. Проверено тестом
  `canonical_bytes_are_stable_across_serialize_round_trips`.
- **Anti-downgrade -- две независимые проверки**: (а) `manifest.version
  > installed_version` (классический анти-даунгрейд, отклоняет и
  повторную установку той же версии); (б) `installed_version >=
  manifest.min_supported_version` (порог совместимости, отдельный от
  (а) -- например, для обновлений, предполагающих, что более ранняя
  миграция уже прошла).
- **Allow-list разделов -- параметр вызова, не константа крейта**.
  `check_installable` принимает `allowed_partitions: &[&str]` --
  ADR-044's `{init_boot_a, vendor_boot_a}` передаётся вызывающей
  стороной (сегодня -- `saai-ota-sign`'s CLI по умолчанию), не
  зашито в библиотеку, чтобы крейт оставался тестируемым независимо от
  того, где именно закреплён список.
- **`check_installable`** -- полный install-time gate: подпись, схема,
  device model, anti-downgrade, allow-list. **`verify_partition_bytes`**
  -- отдельная, более поздняя проверка содержимого одного staged-раздела
  (sha256+размер) -- вызывается после `check_installable`, до записи
  (Change 3/4).
- **CLI `saai-ota-sign`** (build-server-side, никогда не запускается на
  устройстве): `keygen` (генерирует ed25519-пару, приватный ключ --
  файл с правами `0600`), `build` (считает sha256/размер для файлов
  разделов, собирает и подписывает `ManifestBody`), `verify` (гоняет
  `check_installable`/`verify_partition_bytes` против реального
  файла -- та же проверка, что будущий Change 3's device-side
  верификатор будет делать после staged download).

## Test (физически прогнано на R620, хост, устройство не трогалось)

- `cargo test -p saai-ota-manifest` -- 15/15 unit-тестов зелёные:
  валидный манифест принят; испорченная подпись (один hex-символ)
  отклонена; подделанное тело при формально валидном поле подписи
  отклонено; подпись чужим ключом отклонена; чужая модель устройства
  отклонена; downgrade отклонён; повторная установка той же версии
  отклонена как downgrade; версия ниже `min_supported_version`
  отклонена; раздел не из allow-list (`abl_a`, цепочка бутлоадера)
  отклонён; неизвестное поле схемы отклонено на этапе парсинга;
  неподдерживаемая версия схемы отклонена; повреждённый artifact
  отклонён по sha256/размеру; целый artifact принят; раздел,
  отсутствующий в манифесте, отклонён; канонические байты стабильны
  через цикл сериализации.
- **End-to-end на реальных файлах** (не только in-memory фикстуры
  юнит-тестов) -- `saai-ota-sign keygen` + `build` + `verify` в
  `/tmp/ota-e2e` на R620: валидный манифест + валидный artifact --
  принято; чужая модель устройства -- отклонено; downgrade (installed
  версия старше манифеста) -- отклонено; манифест с одним изменённым
  hex-символом подписи -- отклонено (`signature does not verify
  against manifest body`); artifact, подменённый после стейджинга --
  отклонён с точным несовпадением sha256/размера в сообщении об
  ошибке.
- `cargo clippy -p saai-ota-manifest --all-targets` -- ноль
  предупреждений. `cargo fmt -p saai-ota-manifest -- --check` -- чисто
  после одного автоформатирования. `cargo build --workspace` -- весь
  workspace по-прежнему собирается чисто после добавления крейта и
  четырёх новых workspace-зависимостей (`ed25519-dalek`, `rand_core`,
  `sha2`, `hex`).

## Последствия

- Схема подписи и manifest готовы для Change 3 (staged download на
  устройстве) -- device-side верификатор будет вызывать те же
  `check_installable`/`verify_partition_bytes` из этого крейта, не
  переизобретать проверку.
- Приватный ключ, сгенерированный `keygen` в этом раунде --
  тестовый, живёт только в `/tmp/ota-e2e` на R620, не коммитится и не
  используется для реального будущего релиза (per DoR's Threat/privacy
  impact: реальный ключ -- отдельный, вне git, до первого настоящего
  подписанного релиза).
- `check_installable` не проверяет содержимое разделов -- это
  намеренно отдельная функция (`verify_partition_bytes`), потому что
  во время самой проверки манифеста artifact ещё может быть не
  скачан целиком (Change 3's staged download).

## Ссылки

- `docs/os/sprints/S12-ota-release-gate.md` -- Change 2, закрывается
  этим ADR.
- `crates/saai-ota-manifest/src/lib.rs`, `src/bin/saai-ota-sign.rs`.
- ADR-044 -- allow-list `{init_boot_a, vendor_boot_a}`, переданный сюда
  как параметр, не переопределённый.
