# SaaiOS Human Interface Architecture
## Контекстная, AI-native, распределённая персональная вычислительная среда

**Статус документа:** концепт / архитектурная спецификация  
**Назначение:** единый исходный документ для дальнейшего включения идей в product roadmap, UI roadmap, архитектурные ADR и задачи воркеров SaaiOS.  
**Основной принцип:** SaaiOS — не «ещё одна мобильная ОС» и не «Android с ИИ». Это персональная вычислительная среда, где пользователь работает с контекстами, объектами, намерениями и действиями, а устройства становятся узлами единой системы.

---

# 1. Видение

SaaiOS должна уйти от классической модели:

```text
Пользователь
  ↓
Приложение
  ↓
Экран
  ↓
Функция
  ↓
Результат
```

к модели:

```text
Пользователь
  ↓
Контекст
  ↓
Объект / намерение
  ↓
План
  ↓
Исполнение
  ↓
Наблюдение
  ↓
Результат
```

Приложение при этом перестаёт быть владельцем пользовательской работы. Оно становится одним из возможных исполнителей или представлений.

SaaiOS должна воспринимать телефон, ПК, автомобильную систему, сервер, NAS, телевизор, планшет, часы и будущие устройства не как отдельные миры, которые надо синхронизировать, а как **узлы одной персональной среды**.

---

# 2. Базовые сущности интерфейса и архитектуры

В основу SaaiOS предлагается положить следующие фундаментальные сущности:

```text
Space Graph       — контекст пользователя
Entity Graph      — объекты пользовательского мира
Context Frame     — активная комбинация контекстов сейчас
Capability Graph  — что система и доступные узлы умеют
Resource Fabric   — где есть вычисления, хранение и ресурсы
Surface           — где и как информация показывается
Intent            — чего хочет пользователь
Task              — что система должна довести до результата
Action            — конкретное действие
Observation       — факт о состоянии мира/системы
Orb               — универсальная точка взаимодействия
```

Вся GUI-архитектура должна быть построена вокруг этих сущностей.

---

# 3. Главный принцип: не приложение, а объект

SaaiOS должна считать первичным не приложение, а **Entity**.

Примеры Entity:

```text
Person
Task
File
Photo
Conversation
Device
Place
Space
Automation
Project
Server
Vehicle
Event
Application
Model
Document
Media
```

Одна и та же визуальная логика должна применяться к различным типам объектов.

## 3.1 Универсальный Object View

```text
┌──────────────────────────────┐
│ ←  Object                    │
│    Type                      │
├──────────────────────────────┤
│                              │
│        Основное состояние    │
│                              │
├──────────────────────────────┤
│ Related                      │
│                              │
│ связанные объекты            │
│ задачи                       │
│ история                      │
│ события                      │
│                              │
├──────────────────────────────┤
│          Orb / Actions       │
└──────────────────────────────┘
```

Для каждого типа объекта меняется содержимое, но не mental model.

---

# 4. Space как граф контекстов, а не набор рабочих столов

Space не должен быть ограничен фиксированным набором вроде:

```text
Home
Work
Personal
```

Space — это **именованный контекст**, который изменяет релевантность и доступность объектов, действий, памяти и автоматизаций.

Количество Space потенциально не ограничено.

Примеры:

```text
Дом
Работа
Машина
Дача
Метро
Отдых
Отпуск
Проект X
Семья
Киев
Поездка
Встреча
Driving
Morning
Weekend
Offline
Low Battery
```

## 4.1 Space образуют граф

```text
                Work
               /    \
           Office   Project X
             |         |
           Kyiv      Laptop
             \         /
                Metro
                  |
               Commute
                  |
                 Car
                  |
                Home
                  |
                Dacha
```

Связи могут быть:

```text
parent_of
child_of
related_to
contains
usually_with
exclusive_with
activates
deactivates
inherits_from
```

## 4.2 Space не владеет объектами

Не:

```text
Entity belongs to Space
```

а:

```text
Entity ↔ Space
```

many-to-many.

Один объект может быть связан с несколькими контекстами:

```text
Task: Купить масло
Spaces:
- Car
- Dacha
- Weekend
```

## 4.3 Типы Space

Внутренне система может различать:

```text
semantic
  Work
  Personal
  Family
  Project X

physical
  Home
  Car
  Dacha
  Office

activity
  Driving
  Meeting
  Sleeping
  Vacation

temporal
  Morning
  Evening
  Weekend

system
  Offline
  Charging
  Low Battery
  Roaming
```

Пользователь не обязан видеть эти классы.

---

# 5. Active Context и Context Frame

В SaaiOS не должно быть одного `current_space`.

Вместо этого:

```text
ContextFrame:
  Personal     1.00 manual
  Car          0.99 bluetooth
  Driving      0.91 motion
  Evening      1.00 clock
```

Текущий контекст — это набор активных Space с:

```text
space_id
confidence
source
timestamp
priority
```

## 5.1 Источники активации

Space может активироваться по:

```text
manual selection
location
Bluetooth
Wi-Fi
time
calendar
motion
vehicle connection
device presence
user behavior
task state
conversation
automation
AI inference
```

## 5.2 Автоматическое переключение

Переключение Space **не должно требовать подтверждения**, если само изменение контекста не выполняет опасного действия.

Пример:

```text
BT car connected
+ movement detected
→ activate Car
→ activate Driving
```

SaaiOS не должна спрашивать:

> «Переключиться в режим Машина?»

Это шум.

## 5.3 Автоматическое создание Space

AI может создавать новые временные/устойчивые Space.

Примеры:

```text
Поездка в Киев
Конференция
Отпуск 2027
Ремонт кухни
Новый проект
Гостиница
```

Чтобы избежать мусора, Space должны иметь жизненный цикл:

```text
temporary
emerging
stable
archived
```

AI может повышать или понижать устойчивость Space по мере использования.

---

# 6. Space не должен визуально давить на пользователя

Контекст должен быть частью системы, но не превращаться в постоянный ярлык:

```text
YOU ARE IN HOME MODE
YOU ARE IN WORK MODE
```

Это недопустимо.

Пользователь должен ощущать контекст почти подсознательно.

Главный визуальный носитель этого состояния — **цветовой сигнал**.

---

# 7. Context Light / Context Dot

Каждый Space может иметь цвет.

Пример:

```text
Дом      → зелёный
Работа   → голубой
Машина   → янтарный
Личное   → фиолетовый
```

Палитра не должна быть жёстко встроена. Пользователь может менять цвета.

На обычном UI цвет показывается минимально:

```text
09:42                                  ●
```

На Always-On Display:

```text
               09:42

                 ●
```

## 7.1 Семантика визуального языка

Ввести строгую грамматику:

```text
COLOR      = context / Space
SHAPE      = state
MOTION     = activity
ARC/FILL   = quantity / progress
RING       = attention
```

Цвет не должен одновременно означать ошибку, успех, заряд и контекст.

Например красный как «ошибка» нельзя переиспользовать, если красный уже выбран пользователем для определённого Space.

---

# 8. Orb — главный элемент Human Interface

Orb становится фирменным элементом SaaiOS.

Это не просто кнопка AI, не Start Menu и не launcher.

Orb — универсальная точка взаимодействия:

```text
Context → Intent → Action
```

## 8.1 Базовое состояние

```text
●
```

Orb должен быть максимально спокойным.

В норме он не должен отвлекать.

## 8.2 Orb как индикатор

Orb может одновременно нести несколько уровней информации:

```text
цвет         → Space / context
внешняя дуга → battery / progress
пульсация    → active work
кольцо       → attention
```

Например заряд батареи:

```text
     ╭────╮
    │ ●  │
     ╰────╯
      ↑
outer arc = battery
```

Точный процент показывается только по запросу/касанию.

---

# 9. Orb как универсальный контроллер

## 9.1 Tap

Открывает радиальное меню с актуальными действиями.

```text
                  Music
                    ●
                    │
            Camera  │  Navigate
                \   │   /
                 \  │  /
         Calls ──── ◉ ──── Messages
                 /  │  \
                /   │   \
             Home   │   Tasks
                    │
                   More
```

Содержимое зависит от:

```text
current ContextFrame
current object
device capabilities
user habits
time
policy
active tasks
```

## 9.2 Примеры по контексту

### Машина

```text
Navigation
Music
Call
Car status
Messages
Drive home
```

### Работа

```text
Tasks
Calendar
Files
Terminal
Messages
Project
```

### Дача

```text
Weather
Garden
Cameras
Tasks
Music
Home
```

Пользователь может закреплять сектора.

AI может динамически повышать релевантные действия.

---

# 10. Orb и голос

## 10.1 Long press = Push-to-Talk

Зажатие Orb:

```text
●
↓
◉
↓
Listening
```

Пользователь говорит намерение и отпускает Orb.

Это базовый голосовой interaction pattern.

## 10.2 Locked listening

Дополнительный lock-жест может включать длительное прослушивание:

```text
hold
  ↓
slide to lock
  ↓
continuous listening
```

Этот режим должен быть визуально очевидным.

## 10.3 Privacy model

Нужно различать:

```text
Listening
  локальная обработка

Streaming
  аудио покидает устройство
```

Это разные состояния.

Постоянное прослушивание не означает постоянную отправку звука модели.

Предпочтительная архитектура:

```text
microphone
  ↓
local lightweight event detector
  ↓
speech/activity detection
  ↓
optional local transcription
  ↓
intent detector
  ↓
large model only if needed
```

---

# 11. Orb как «кубик Рубика»

Orb должен поддерживать жесты, но не превращаться в коллекцию скрытых комбинаций.

Не должно быть модели:

```text
swipe left = X
swipe right = Y
double tap = Z
triple tap = ...
hold 0.5 sec = ...
hold 2 sec = ...
```

Пользователь не должен запоминать десятки жестов.

## 11.1 Модель слоёв

Первый жест выбирает измерение.

Второй управляет им.

Пример:

```text
                    SYSTEM
                      ↑

        CONTEXT  ←   ●   →  ACTIONS

                      ↓
                  ATTENTION
```

После входа в слой Orb превращается в контроллер параметра.

Пример громкости:

```text
            Volume

←────────────●────────────→
0                        100
```

Пример яркости:

```text
Brightness
──────────────
58%
```

Таким образом Orb ведёт себя как виртуальный trackball / Digital Crown / command wheel.

---

# 12. Orb как вход в Object Graph

Если открыт конкретный объект, Orb становится контекстным.

## Фото

```text
Share
Edit
Ask
Remember
Related
Delete
```

## Контакт

```text
Call
Message
Schedule
Files
Tasks
Ask
```

## Сервер

```text
Status
Logs
SSH
Restart
Metrics
Diagnose
```

Таким образом Orb реализует универсальную схему:

```text
OPEN
ACT
ASK
SHARE
RELATE
SCHEDULE
AUTOMATE
REMEMBER
INSPECT
```

---

# 13. Now — главный экран, но не launcher

Now должен показывать только текущее состояние пользователя и системы.

Пример:

```text
09:42                                  74%

Сегодня
────────────────────────────
10:30  Daily
14:00  Позвонить Андрею

Продолжается
────────────────────────────
● Подготовка отчёта
  3 из 5 действий выполнено

Требует внимания
────────────────────────────
! Обновление готово

                         ●
```

Если ничего нет:

```text
09:42                                  74%

                Сейчас

           Ничего срочного


                         ●
```

Пустой интерфейс считается нормальным.

SaaiOS не должна искусственно заполнять экран.

---

# 14. Главная mental model пользователя

Пользователю достаточно понимать пять вещей:

```text
NOW
что происходит сейчас

SPACE
в каком контексте я нахожусь

OBJECT
с чем я работаю

INTENT
что я хочу получить

ACTION
что система собирается сделать
```

---

# 15. Flow пользовательского действия

```text
             SPACE
               │
              NOW
               │
            OBJECT
               │
             INTENT
               │
              PLAN
               │
       ┌───────┴────────┐
       │                │
     ACTION          CONFIRM
       │                │
       └───────┬────────┘
               ▼
            RESULT
               │
             EVENT
               │
            MEMORY
```

---

# 16. Минимум подтверждений

SaaiOS не должна превращаться в систему из тысячи permission dialogs.

Подтверждение нужно не для «действия системы», а для **рискованного последствия**.

Не требуют подтверждения:

```text
смена Space
создание временного Space
обновление релевантности
изменение сортировки
локальный анализ
переключение presentation
выбор compute node
```

Могут требовать подтверждения:

```text
отправка данных наружу
покупка
удаление
изменение безопасности
доступ к чувствительным данным
запуск опасного system action
выдача постоянного разрешения
```

---

# 17. Состояние функции: ABSENT / UNAVAILABLE / DENIED

UI должен различать три состояния.

## ABSENT

Функции физически нет.

```text
capability.camera.capture отсутствует
→ Camera UI не показывается
```

## UNAVAILABLE

Функция существует, но временно недоступна.

```text
camera service crashed
→ Camera показывается как unavailable
```

## DENIED

Функция есть, но policy запрещает доступ.

```text
camera capability denied
→ UI показывает, что доступ запрещён
```

---

# 18. Декларативный UI через capabilities

Каждая функция UI должна иметь requirements.

Пример:

```text
Camera UI
requires:
  capability.camera.capture
```

```text
Calls
requires:
  capability.telephony.call
```

```text
Voice Intent
requires:
  capability.audio.capture
  capability.speech.recognition
```

Нет capability — нет элемента интерфейса.

Появилась capability — соответствующий UI может появиться без переделки shell.

---

# 19. Multi-device: не синхронизация устройств, а одна среда

Классические системы:

```text
Phone
Laptop
Car
Tablet

↓ sync
↓ sync
↓ sync
```

SaaiOS:

```text
                    USER
                      │
               Personal Graph
                      │
        ┌─────────────┼─────────────┐
        │             │             │
      Spaces       Entities       Memory
        │             │             │
        └─────────────┼─────────────┘
                      │
                Context Engine
                      │
                 Task Engine
                      │
                 Capability
                    Graph
                      │
          ┌───────────┼───────────┐
          │           │           │
        Phone       Desktop       Car
```

Устройства не являются отдельными мирами.

Они показывают разные проекции одного пользовательского состояния.

---

# 20. Surface вместо «устройства»

Ввести понятие **Surface**.

Surface — место/способ представления.

Примеры:

```text
Pixel main display
Pixel AOD
Desktop monitor
TV
Car display
Watch
AR glasses
Audio
Haptics
Voice
```

Один device может предоставлять несколько Surface.

Пример Pixel:

```text
Pixel 7
├── Main display
├── AOD
├── Audio
├── Haptics
└── Orb
```

---

# 21. Presentation должен адаптироваться к Surface

Один объект не обязан выглядеть одинаково везде.

AOD:

```text
        02:04

          ●
```

Телефон:

```text
Now
Task
Context
Orb
```

Desktop:

```text
Projects | Current Work | Related Objects
                         ●
```

Car:

```text
Navigation
18 min
●
```

Контекст и объекты остаются едиными, presentation меняется.

---

# 22. Capability Graph

У каждого узла есть capabilities.

Телефон:

```text
camera
gps
microphone
cellular
touch
haptics
display.small
```

ПК:

```text
keyboard
mouse
display.large
gpu
compute.high
storage.large
```

Машина:

```text
display.medium
audio
gps
vehicle.speed
vehicle.battery
steering.controls
```

SaaiOS должна спрашивать не:

> Где находится приложение?

а:

> Какие способности доступны сейчас?

---

# 23. Personal Resource Fabric

SaaiOS должна рассматривать все устройства пользователя как персональный вычислительный кластер.

Пример:

```text
                    Personal Fabric
                         │
        ┌────────────────┼────────────────┐
        │                │                │
      Pixel          Workstation       Laptop
        │                │                │
     Camera           RTX GPU            SSD
     GPS              High CPU           Screen
     Mic              High RAM           Keyboard
        │                │                │
        └────────────────┼────────────────┘
                         │
                  Capability Graph
                         │
                     Scheduler
```

---

# 24. Compute Node

Любой вычислительный участник — Compute Node.

Это может быть:

```text
Phone
Desktop
NAS
Server
Car
Raspberry Pi
GPU box
Cloud VM
```

Пример описания:

```text
ComputeNode:
  identity
  owner
  trust
  capabilities
  resources
  location
  availability
  connectivity
  power_state
  policy
```

---

# 25. Scheduler

SaaiOS должна автоматически выбирать, где выполнять задачу.

Не через SSH как пользовательскую модель:

```text
Pixel
  ssh home-pc
  run command
```

а:

```text
Task
 ↓
Scheduler
 ↓
Action
 ↓
eligible nodes
 ↓
best node
 ↓
Executor
```

---

# 26. Пример распределённого исполнения

Запрос:

> Проанализируй 300 фотографий и найди дубликаты.

План:

```text
Pixel
  capture / source objects

NAS
  storage

Home Workstation
  embeddings
  vision
  clustering

Pixel
  final presentation
```

Для пользователя это одна задача.

---

# 27. Выбор узла не только по мощности

Scheduler должен учитывать:

```text
compute_time
network_latency
transfer_cost
battery_cost
privacy_cost
monetary_cost
thermal_cost
availability
trust
data policy
```

Условная scoring model:

```text
score =
    compute_time
  + network_latency
  + transfer_cost
  + battery_cost
  + privacy_cost
  + monetary_cost
  + thermal_cost
```

Пример:

```text
simple local command
→ Pixel

heavy LLM
→ Home Workstation

secret work document
→ Work node only

cloud
→ only if allowed by policy
```

---

# 28. Policy-aware compute

Пример правила:

```text
if space == Work:
    deny nodes tagged PersonalHome
```

Пример:

> Никогда не отправляй рабочие документы на домашний компьютер.

Это должна быть policy, а не рекомендация модели.

Другой пример:

```text
prefer Home-PC
when:
  mains_power == true
  idle == true
```

---

# 29. Tiered Intelligence

AI не должна всегда означать большую модель.

Рекомендуемый уровень:

```text
Tier 0
deterministic rules
no AI

Tier 1
tiny local classifier

Tier 2
small local LLM

Tier 3
LAN / personal compute

Tier 4
cloud model
```

Простые системные команды должны обходиться без LLM.

---

# 30. AI не является источником истины о системе

Сохраняется строгий принцип:

```text
Observation
    ↓
Action
    ↓
Verification
```

LLM:

```text
proposes
```

Policy:

```text
authorizes
```

Executor:

```text
acts
```

Observation:

```text
verifies
```

Audit:

```text
records
```

AI не может сама считать действие успешным.

---

# 31. Orb как общая идентичность между устройствами

На телефоне:

```text
●
```

На desktop:

```text
●
```

В машине:

```text
●
```

На TV:

```text
●
```

Логически это один интерфейсный элемент одной среды.

Функции вокруг Orb меняются по Surface и ContextFrame.

---

# 32. Orb не должен быть «новогодней ёлкой»

Это важное правило.

По умолчанию:

```text
●
```

Дополнительная информация появляется только при необходимости.

Нельзя одновременно показывать:

```text
цвет
5 колец
3 мигания
иконки
текст
процент
анимации
```

Минимализм здесь функциональный, а не декоративный.

---

# 33. Основные визуальные принципы SaaiOS

## 33.1 Calm UI

Интерфейс спокоен.

Он не требует внимания без причины.

## 33.2 Progressive disclosure

Сначала минимум.

Подробности только по запросу.

## 33.3 Context without pressure

Система знает контекст, но не напоминает о нём постоянно.

## 33.4 Reversible actions

Максимум операций должны быть обратимыми.

## 33.5 Ambient intelligence

Система может действовать автоматически, если действие безопасно и обратимо.

## 33.6 No app-grid dependency

Сетка приложений может существовать как fallback, но не должна быть главным mental model.

---

# 34. Навигационная карта высокого уровня

```text
SaaiOS
│
├── NOW
│   ├── Current state
│   ├── Attention
│   ├── Active work
│   ├── Upcoming
│   └── Intent
│
├── SPACES
│   ├── active context
│   ├── related spaces
│   └── context history
│
├── OBJECTS
│   ├── People
│   ├── Tasks
│   ├── Files
│   ├── Conversations
│   ├── Places
│   ├── Devices
│   └── Projects
│
├── COMMUNICATION
├── TIME
├── MEDIA / SENSORS
├── DEVICE
├── INTELLIGENCE
├── APPS
└── SETTINGS
```

Но это **информационная архитектура**, а не обязательно видимое меню.

---

# 35. Что реально должен видеть пользователь

Главная модель может быть сведена до:

```text
                  NOW
                   │
             current object
                   │
                   ●
                  ORB
              ╱    │    ╲
         Context  Intent  Actions
             │      │       │
           Space   AI     Direct
             │      │       │
             └──────┴───────┘
                    │
                  RESULT
```

---

# 36. Что пользователь не должен знать

Обычному пользователю не нужно понимать:

```text
daemon
service
provider
model endpoint
socket
IP
port
scheduler
node routing
capability registry
policy engine
entity store
```

Это debug / developer surfaces.

---

# 37. Developer / Expert mode

Для разработчиков SaaiOS должна позволять раскрыть технический слой:

```text
active ContextFrame
capabilities
node selection
task plan
action graph
policy decision
observations
audit trail
latency
resource cost
```

Но это не должно загрязнять обычный интерфейс.

---

# 38. AOD

Always-On Display должен быть частью Surface-модели.

Минимальный AOD:

```text
          09:42

            ●
```

Допустимые дополнительные состояния:

```text
● normal
◉ active processing
◎ attention
```

Нельзя превращать AOD в второй lock screen с кучей карточек.

---

# 39. Батарея через Orb

Батарея может отображаться дугой Orb.

Принцип:

```text
context color = fill/color
battery = outer arc
```

При обычном использовании пользователь видит приблизительное состояние.

Точный процент доступен при раскрытии.

---

# 40. Взаимодействие между Surface

Пример:

> Продолжить на компьютере.

SaaiOS должна передавать не пиксели и не обязательно файл, а:

```text
Entity reference
Task reference
Presentation state
Context
Required capabilities
```

Desktop сам строит подходящее presentation.

---

# 41. Apps как исполнители

В SaaiOS приложение не должно владеть объектом.

Модель:

```text
                   USER GRAPH
                       │
          ┌────────────┼────────────┐
          │            │            │
       Objects       Tasks       Conversations
          │            │            │
          └────────────┼────────────┘
                       │
                 Applications
                  are tools
```

---

# 42. Синхронизация

Разделить четыре понятия:

```text
SYNC
состояние и объекты

EXECUTION
выполнение задач

CAPABILITY SHARING
использование возможностей другого узла

PRESENTATION
отображение состояния
```

Эти механизмы не должны смешиваться.

---

# 43. Trust model

У каждого Compute Node должен быть trust level.

Например:

```text
personal_trusted
personal_limited
work_managed
guest
cloud
unknown
```

Policy должна учитывать trust.

---

# 44. Identity и peer fabric

Узлы должны иметь криптографическую identity.

Пользовательский UX:

```text
My devices

● Pixel
● Home PC
● NAS
○ Laptop
```

Под капотом может быть mesh/overlay network.

Пользователь не должен видеть IP и VPN, если не открывает экспертный режим.

---

# 45. Offline first

Если Resource Fabric частично недоступен:

```text
Home PC offline
```

SaaiOS должна:

1. выбрать другой узел;
2. деградировать задачу;
3. выполнить локально;
4. отложить;
5. только затем спрашивать пользователя, если без него нельзя принять решение.

---

# 46. Fail-safe / degraded mode

Без LLM система должна оставаться пригодной к использованию.

Должны работать:

```text
Files
Settings
Network
Calls
Tasks
Direct actions
Device controls
Recovery
Terminal/developer tools
```

AI ускоряет работу, но не является единственным control plane.

---

# 47. System surfaces

Системные поверхности:

```text
Boot
Lock
AOD
Now
Overview
Notifications
Quick Controls
Object View
Context Sheet
Decision Overlay
Recovery
Developer View
```

---

# 48. Context Sheet

Универсальный contextual overlay.

Пример файла:

```text
image.jpg

Open
Share
Ask about this
Add to task
Move to Space
Remember
Automate
Details
```

Пример Wi-Fi:

```text
Home-5G

Disconnect
Forget
Diagnose
Share
Automate
Details
```

---

# 49. Context History

Пользователь должен иметь возможность увидеть:

```text
09:10 Home
09:42 Car
10:18 Work
18:02 Car
18:46 Home
```

Но эта история не должна постоянно показываться.

Она нужна для:

```text
debug
automation
privacy review
context corrections
```

---

# 50. Corrections and learning

Если AI ошиблась:

> Я не в машине.

Пользователь исправляет контекст.

Система должна запомнить correction signal.

Не нужно показывать длинный ML-интерфейс.

---

# 51. Автоматизации

Automation может быть связана с Space.

Пример:

```text
when:
  Car active
  AND Driving active

then:
  show Navigation shortcut
  prefer voice UI
  reduce visual notifications
```

Другой:

```text
when:
  Home
  AND Night

then:
  reduce brightness
  prioritize quiet actions
```

---

# 52. Accessibility

Orb не должен полагаться только на цвет.

Нужны альтернативы:

```text
shape
haptic pattern
position
text label on request
high contrast
screen reader semantics
```

Для color-blind users контекст должен быть различим не только цветом.

---

# 53. Безопасность continuous listening

Continuous listening должен:

```text
иметь явный visual state
иметь local/off-device distinction
иметь fast stop gesture
логироваться
уважать per-Space policy
уметь отключаться hardware/privacy policy
```

---

# 54. UX правило: AI не задаёт лишних вопросов

AI должна предпочитать:

```text
safe assumption
reversible action
silent context update
background execution
```

вместо:

```text
confirm?
confirm?
confirm?
```

Но при высокой цене ошибки должна спрашивать.

---

# 55. UI state machine Orb

Минимально:

```text
IDLE
MENU
LISTENING
LISTEN_LOCKED
ACTIVE_TASK
ATTENTION
CONTROL_LAYER
OBJECT_CONTEXT
```

Каждое состояние должно иметь:

```text
visual form
gesture map
haptic response
accessibility description
exit gesture
```

---

# 56. MVP Orb

Первый практический MVP:

```text
tap
→ radial menu

long press
→ push-to-talk

hold + lock
→ continuous listen

swipe
→ Context / Actions / Attention / System

outer arc
→ battery

color
→ primary active Space
```

Не добавлять больше до пользовательского тестирования.

---

# 57. MVP Space Engine

Минимальная версия:

```text
manual Space
Wi-Fi activation
Bluetooth activation
time activation
simple AI suggestion
ContextFrame
history
color mapping
```

После этого:

```text
location
motion
behavioral patterns
calendar
device presence
automatic creation
```

---

# 58. MVP Resource Fabric

Этап 1:

```text
trusted nodes
node discovery
capability advertisement
secure channel
manual task target
```

Этап 2:

```text
automatic scheduler
resource metrics
policy-aware placement
fallback
```

Этап 3:

```text
distributed task execution
multi-node pipelines
resource borrowing
```

---

# 59. Возможная внутренняя модель

```text
Human
  │
  ▼
Orb / Surface
  │
  ▼
Context Engine
  │
  ├── Space Graph
  └── ContextFrame
  │
  ▼
Entity Graph
  │
  ▼
Intent
  │
  ▼
Work Scheduler
  │
  ├── Planner
  ├── Policy
  ├── Capability Resolver
  └── Resource Scheduler
  │
  ▼
Action
  │
  ▼
Executor
  │
  ▼
Observation
  │
  ▼
Verification
  │
  ▼
Event / Memory / Audit
```

---

# 60. Архитектурный инвариант

Ни UI, ни AI не должны обходить:

```text
Policy
Verification
Audit
```

Особенно для:

```text
destructive actions
security changes
external communication
financial actions
persistent permission changes
```

---

# 61. Product identity

SaaiOS должна иметь узнаваемую интерфейсную идентичность.

Ключевые элементы:

```text
Orb
Context Light
Calm surfaces
Object-centric UI
Ambient Context
No app-grid dependency
Cross-device continuity
```

---

# 62. Фирменная идея Orb

Orb можно воспринимать как:

```text
курсор эпохи AI-native UI
```

Мышь когда-то стала естественным манипулятором GUI.

Orb может стать естественным манипулятором:

```text
context
intent
actions
continuous controls
voice
attention
```

---

# 63. Принцип персональной распределённой ОС

SaaiOS должна мыслить так:

> «У пользователя есть вычислительная среда.»

а не:

> «У пользователя есть телефон, ноутбук и домашний ПК.»

Новый узел просто присоединяется к Personal Fabric.

Система получает дополнительные:

```text
compute
storage
sensors
surfaces
network access
specialized hardware
```

и автоматически становится мощнее.

---

# 64. Пример «домашний GPU как мозг»

Home Workstation:

```text
high CPU
powerful GPU
large RAM
large storage
mains power
```

автоматически может брать:

```text
LLM inference
image generation
speech recognition
video processing
embeddings
search indexing
builds
backups
```

Телефон:

```text
capture
authentication
sensors
UI
voice
quick local actions
```

Для пользователя это одна система.

---

# 65. Product rules для воркеров

При внедрении любой новой функции проверять:

1. Это объект, действие, capability или отдельное приложение?
2. Может ли функция быть представлена через существующий Object View?
3. Требует ли она нового UI или только новой capability?
4. Как она влияет на ContextFrame?
5. Нужно ли пользователю видеть её постоянно?
6. Можно ли действие сделать без confirmation?
7. Можно ли его безопасно отменить?
8. Где должен выполняться action?
9. Может ли Resource Fabric выполнить его лучше?
10. Как оно выглядит на разных Surface?
11. Что происходит offline?
12. Что происходит без AI?
13. Как это логируется?
14. Как это проверяется Observation?
15. Не превращаем ли мы Orb в перегруженный контроллер?

---

# 66. Чего делать нельзя

Нельзя превращать SaaiOS в:

```text
Android launcher clone
chatbot with system permissions
app grid with AI button
dashboard of widgets
OS that only works when cloud AI is reachable
UI where every action asks confirmation
distributed shell over SSH pretending to be distributed OS
```

---

# 67. Целевая формула

```text
             HUMAN
               │
              ORB
               │
            CONTEXT
               │
          SPACE GRAPH
               │
          ENTITY GRAPH
               │
             INTENT
               │
              TASK
               │
           SCHEDULER
       ╱       │       ╲
   POLICY  CAPABILITY  RESOURCES
       ╲       │       ╱
          RESOURCE FABRIC
      ╱        │        ╲
   PHONE     HOME PC    CLOUD
      ╲        │        ╱
            SURFACES
```

---

# 68. Итог

SaaiOS должна стать не новой оболочкой телефона, а новым способом организации персональных вычислений.

Ключевые идеи:

- **Space — контекстный граф**, а не папка и не профиль.
- **ContextFrame — активная комбинация Space**, а не один режим.
- **AI может переключать и создавать Space автоматически**, если это безопасно.
- **Контекст показывается минимально**, прежде всего через Context Light.
- **Orb — фирменный системный манипулятор SaaiOS**.
- **Orb объединяет меню, голос, быстрые действия, контролы и состояние**.
- **Now — главный экран состояния**, а не launcher.
- **Object View — универсальная единица работы**.
- **Apps становятся инструментами**, а не владельцами данных.
- **Surface отделяется от Device**.
- **Capability Graph описывает доступные способности системы**.
- **Resource Fabric превращает все устройства пользователя в персональный вычислительный кластер**.
- **Scheduler автоматически выбирает лучший узел для выполнения задач**.
- **Policy определяет, где данные и действия допустимы**.
- **AI предлагает и планирует, но не является источником системной истины**.
- **Observation → Action → Verification остаётся фундаментальным инвариантом**.
- **SaaiOS должна быть полезной без LLM и без облака**.
- **Пользователь должен ощущать одну вычислительную среду, а не набор синхронизируемых устройств**.

Это и есть направление, в котором SaaiOS может сформировать собственную Human Interface Architecture вместо наследования парадигм Android, iOS, Windows и macOS.

---

# 69. Следующий шаг для roadmap

Рекомендуется разбить дальнейшую работу минимум на следующие инициативы:

```text
HIA-01  Space Graph v2
HIA-02  ContextFrame Engine
HIA-03  Context Light / color language
HIA-04  Orb interaction model
HIA-05  Orb radial menu
HIA-06  Push-to-Talk / listening states
HIA-07  Universal Object View
HIA-08  Surface abstraction
HIA-09  Capability-driven UI
HIA-10  Personal Fabric identity/discovery
HIA-11  Compute Node model
HIA-12  Resource Scheduler
HIA-13  Policy-aware placement
HIA-14  Cross-device presentation continuity
HIA-15  AOD minimal surface
HIA-16  Accessibility semantics
HIA-17  Context history / corrections
HIA-18  Trust model
HIA-19  Offline / degraded operation
HIA-20  Developer diagnostics surface
```

Каждая инициатива должна завершаться не только кодом, но и:

```text
architecture decision
testable behavior
device/host verification
failure behavior
security implications
UX acceptance criteria
```

---

# 70. Короткий тезис для команды

> **SaaiOS — это персональная распределённая вычислительная среда, где контекст задаётся Space Graph, пользователь взаимодействует через Orb, работа хранится в Entity/Task Graph, а любые доступные устройства становятся поверхностями, сенсорами и вычислительными узлами одной системы.**
---

# 71. Уточнение категории продукта: Personal Computing Environment

После дальнейшей проработки термин «персональный компьютер» следует считать слишком узким.

SaaiOS проектируется как **операционная система персональной вычислительной среды**:

> **SaaiOS — Personal Computing Environment (PCE): одна персональная вычислительная среда, которая следует за человеком и динамически использует доступные ему данные, вычисления, устройства, сенсоры, исполнительные механизмы и поверхности.**

Телефон, ПК, сервер или автомобиль не являются центром системы. Они являются участниками среды.

```text
                         USER
                          │
                 Personal Computing
                     Environment
                          │
       ┌──────────────────┼──────────────────┐
       │                  │                  │
     WORLD             RESOURCES          INTERFACE
       │                  │                  │
   Entities             Compute           Surfaces
   Spaces               Storage             Orb
   Memory               Network            Voice
   Tasks                Sensors            Displays
   History              Actuators           Haptics
       │                  │                  │
       └──────────────────┼──────────────────┘
                          │
                       SaaiOS
```

## 71.1 Основной product law

> **The user chooses WHAT. SaaiOS resolves WHERE, HOW and ON WHAT.**

И второй инвариант:

> **The user owns the state. Devices provide capabilities.**

---

# 72. Device Transparency

Физическое устройство является деталью исполнения, пока его физическая природа не важна для намерения.

Пользователь должен говорить:

```text
Открой договор.
Распечатай документ дома.
Покажи сегодняшние фотографии.
Продолжи игру.
```

а не:

```text
Скачай файл на телефон.
Передай его на ПК.
Открой общую папку.
Подключись к домашнему серверу.
Скопируй на устройство.
```

Если SaaiOS вынуждает пользователя регулярно думать о расположении байтов или маршруте между его собственными устройствами, abstraction layer считается неудачным.

## 72.1 Когда устройство всё-таки важно

```text
Включи фонарик на телефоне.
Сфотографируй это.
Выключи компьютер в спальне.
Покажи на телевизоре.
```

Здесь физический узел или Surface являются частью intent и должны уважаться.

---

# 73. Location Transparency для данных

Пользовательская сущность не должна идентифицироваться физическим pathname.

Не:

```text
/storage/emulated/0/Download/report.pdf
```

а:

```text
Entity: document/report
```

Физические копии являются implementation detail:

```text
Document A82F

logical identity:
  A82F

replicas:
  Pixel       cached
  Home NAS    full
  Laptop      unavailable

history:
  version 12

policy:
  personal
```

## 73.1 Logical identity

Entity должна иметь стабильную логическую identity независимо от:

```text
filename
path
device
replica
transport
application
```

Это позволяет ссылаться на объект из Task, Conversation, Space и Memory без привязки к физическому месту хранения.

---

# 74. Replica Manager

Для реализации Location Transparency нужен системный Replica Manager.

Его обязанности:

```text
locate content
materialize replica
cache content
evict cache
verify integrity
resolve versions
replicate according to policy
track availability
```

Пользователь видит:

```text
Available
Available offline
Unavailable
```

Экспертный режим может показывать:

```text
nodes
hashes
versions
paths
encryption
replication state
```

## 74.1 Content addressing

Для immutable/blob-подобных данных предпочтительно использовать content identity:

```text
content_hash
size
mime
encryption metadata
```

Логическая Entity при этом ссылается на одну или несколько версий content.

Это уменьшает дубликаты и позволяет проверять передачу между узлами.

---

# 75. Runtime Transparency

Следующий фундаментальный принцип:

> Пользователь выбирает программу, игру или действие, а не платформенную сборку.

Пользователь не должен в обычном случае решать:

```text
Windows или Linux?
ARM64 или x86-64?
native или Wine?
container или VM?
локально или удалённо?
```

Эти вопросы принадлежат Runtime Resolver.

---

# 76. Software Entity

Программа должна быть объектом Personal Graph:

```text
SoftwareEntity:
  identity
  title
  publisher
  licenses
  entitlements
  available_artifacts
  requirements
  data
  settings
  runtime_history
  trust
```

Пользователь делает:

```text
Add / Install / Play / Open
```

SaaiOS выбирает execution strategy.

---

# 77. Runtime Resolver

Возможные стратегии:

```text
native
compatibility layer
container
VM
emulation
remote execution
web runtime
alternative implementation
```

Пример:

```text
Software
    │
    ▼
Compatibility Resolver
    │
    ├── artifacts
    ├── architecture
    ├── API/runtime requirements
    ├── GPU requirements
    ├── DRM/license
    └── input/display requirements
    │
    ▼
Runtime candidates
    │
    ▼
Resource Scheduler
```

## 77.1 AI-assisted compatibility

Неизвестная программа может проходить контролируемый цикл:

```text
inspect
→ propose runtime
→ sandbox
→ install
→ launch
→ observe
→ diagnose
→ retry
→ verify
```

LLM не имеет права объявлять установку успешной без Observation.

---

# 78. Installation as intent

Пользовательская операция:

```text
"Установить игру"
```

может внутри означать:

```text
resolve entitlement
resolve artifact
verify source/signature
choose runtime
choose node
reserve storage
download
install dependencies
configure sandbox
launch smoke test
register SoftwareEntity
```

Пользователь не должен подтверждать каждый внутренний шаг.

Meaningful confirmation требуется для:

```text
purchase
subscription
material privacy change
dangerous privilege grant
external data disclosure
```

---

# 79. Purchase boundary

Финансовое действие является отдельной trust boundary.

Пример:

```text
Game — 1499 UAH

[Cancel] [Buy]
```

После подтверждения покупки SaaiOS может выполнить технические шаги установки в пределах policy без каскада бессмысленных подтверждений.

---

# 80. Execution Surface и Presentation Surface независимы

Ключевой принцип для игр, тяжёлых приложений и вычислений:

```text
execution surface/node != presentation surface
```

Игра может исполняться на Home GPU Node, а представляться на Pixel.

```text
Home PC:
  game process
  GPU render
  encoder

Pixel:
  video surface
  audio
  touch
  gyro
  microphone
  controller input
```

При переходе на TV:

```text
execution:
  Home PC

presentation:
  TV

input:
  gamepad
```

Сам workload не обязан мигрировать.

---

# 81. Remote execution не является отдельной пользовательской функцией

В интерфейсе не должно быть обязательного понятия:

```text
Remote Play
Remote Desktop
Remote App
```

Пользователь выбирает:

```text
Play
Open
Run
```

Удалённое исполнение — одна из стратегий Scheduler.

При необходимости Expert View может показать фактический execution node.

---

# 82. Continuity

Task, SoftwareEntity и Entity должны переживать смену Surface.

Пример:

```text
Phone
  ↓
start game
  ↓
Home GPU executes
  ↓
walk to TV
  ↓
presentation moves to TV
  ↓
sit at desktop
  ↓
presentation moves to monitor
```

Не требуется создавать новую пользовательскую session только из-за смены устройства.

---

# 83. Personal Fabric не имеет обязательного master device

Архитектурно:

```text
NO MASTER DEVICE
```

Есть:

```text
User Identity
Personal Graph
Policy
Resource Fabric
ContextFrame
available Nodes
available Surfaces
```

Pixel может быть главным присутствующим рядом устройством, но не должен быть единственной точкой существования среды.

Потеря одного узла не должна уничтожать Personal Computing Environment.

---

# 84. Environment Membership

Новый узел присоединяется к среде, а не «синхронизируется с телефоном».

После доверенного enrollment он публикует:

```text
identity
capabilities
resources
surfaces
trust
availability
policy attributes
```

Пример:

```text
New GPU box joined

+ compute.gpu
+ gpu.vram.32gb
+ video.encode
```

SaaiOS автоматически получает новые execution strategies.

---

# 85. Resource Transparency

Третья прозрачность:

```text
DEVICE TRANSPARENCY
"На каком устройстве?"

LOCATION TRANSPARENCY
"Где лежат данные?"

RUNTIME TRANSPARENCY
"Для какой ОС/архитектуры?"

RESOURCE TRANSPARENCY
"Где это вычисляется?"
```

Пользователь должен сталкиваться с этими деталями только тогда, когда они влияют на цену, безопасность, задержку, качество или явно интересуют его.

---

# 86. Personal Fabric как рынок возможностей

Scheduler не должен видеть только машины.

Он должен видеть предложения capabilities:

```text
compute.cpu
compute.gpu
storage
display
camera
microphone
speaker
printer
scanner
network.egress
model.inference
video.encode
location
vehicle.*
```

Задача публикует requirements.

```text
Action requirements
        │
        ▼
Capability Graph
        │
        ▼
eligible providers
        │
        ▼
Policy filter
        │
        ▼
Resource scoring
        │
        ▼
lease capability
```

Это позволяет одинаково маршрутизировать вычисление, печать, камеру или модель.

---

# 87. Capability Leasing

Для распределённой среды полезна концепция временной аренды capability.

Например Task получает:

```text
lease:
  capability: gpu.compute
  provider: Home-PC
  scope: Task-938
  expires: task completion
```

Это лучше, чем выдавать постоянный неограниченный доступ одного узла к другому.

Lease должен быть:

```text
scoped
revocable
auditable
time-bounded
policy-controlled
```

---

# 88. Placement Constraints

Task/Action может задавать не конкретный узел, а ограничения:

```text
requires:
  gpu.vram >= 12GB

must:
  data remain personal

prefer:
  mains powered
  low monetary cost
  low latency

avoid:
  battery powered
  metered network
```

Scheduler сам выбирает placement.

---

# 89. Data Gravity

При выборе execution node нужно учитывать стоимость перемещения данных.

Пример:

```text
2 TB video dataset already on NAS
```

может быть выгоднее обработать рядом с NAS, чем передавать dataset на формально более мощный удалённый GPU.

Scheduler должен учитывать:

```text
data locality
transfer size
bandwidth
latency
energy
privacy
```

---

# 90. Locality hierarchy

Предлагаемая логика предпочтений, если policy не говорит иначе:

```text
same process/device
↓
same trusted LAN
↓
personal remote fabric
↓
trusted external/work infrastructure
↓
cloud
```

Это не жёсткое правило: мощность, latency и data gravity могут изменить выбор.

---

# 91. Network Partition Semantics

Распределённая PCE должна считать сетевой разрыв нормальным состоянием.

Нельзя проектировать систему с предположением:

```text
all nodes always reachable
```

Task должен иметь состояние:

```text
waiting_for_resource
degraded
retryable
migratable
blocked
```

При partition SaaiOS должна понимать, можно ли:

```text
continue locally
resume later
select another node
use stale replica
ask user
```

---

# 92. Durable Task Ownership

Task не должен принадлежать процессу LLM или конкретному устройству.

```text
Task identity
    ↓
durable Work Scheduler
    ↓
leases / actions
```

Перезапуск:

```text
LLM
executor
phone
home PC
network
```

не должен уничтожать состояние долгой задачи.

---

# 93. Idempotency и distributed actions

В распределённой среде повтор команды неизбежен.

Action должен по возможности иметь:

```text
action_id
idempotency_key
preconditions
expected effect
verification
compensation
```

Особенно:

```text
print
purchase
message.send
file.move
device.control
```

Нельзя дважды купить товар или отправить сообщение из-за network retry.

---

# 94. Ownership, Authority, Availability — разные понятия

Не смешивать:

```text
Ownership
кто владеет объектом

Authority
кто имеет право выполнить действие

Availability
где объект/capability доступен сейчас
```

Home PC может иметь replica документа, но не иметь права отправить его наружу.

Work node может иметь compute, но policy может запрещать personal workload.

---

# 95. Provenance

Каждый важный результат должен уметь ответить:

```text
откуда взялись данные?
какой узел выполнял?
какой runtime?
какая модель?
какие actions?
какая версия объекта?
что было verified?
```

Обычный пользователь этого не видит постоянно.

Но provenance критична для:

```text
debug
security
reproducibility
trust
AI-generated results
```

---

# 96. Cost Model

Resource Scheduler должен понимать не только техническую возможность, но и стоимость:

```text
money
battery
energy
bandwidth
latency
privacy
thermal load
wear
```

Например пользователь может задать:

```text
Never use paid cloud compute automatically.
```

или:

```text
Use home GPU whenever practical.
```

---

# 97. QoS / Intent Classes

Разные задачи имеют разные требования.

```text
interactive
  UI / game / voice

near-real-time
  transcription / video

background
  indexing / backup

batch
  rendering / builds

critical
  security / recovery
```

Scheduler должен учитывать класс.

Для игры latency важнее энергоэффективности.

Для ночного индексирования наоборот.

---

# 98. Compute Presence

UI может по запросу показывать, где исполняется активная работа:

```text
Rendering video

Home Workstation
GPU 74%
12 min
```

Но placement не должен становиться обязательной частью обычного UX.

---

# 99. Handoff vs Migration

Различать:

```text
Handoff
перенос presentation/input

Migration
перенос самого execution state
```

Для большинства приложений сначала достаточно Handoff.

Настоящая process migration сложнее и не должна быть обязательным условием PCE.

---

# 100. Device loss

Если устройство потеряно:

```text
revoke identity
revoke leases
rotate affected secrets
invalidate sessions
remove sensitive cached replicas
```

Personal Environment продолжает существовать на остальных доверенных узлах.

---

# 101. Backup как свойство Entity Graph

Пользователь не должен вручную выбирать тысячи каталогов для backup.

Policy может описываться семантически:

```text
Photos:
  keep 2 durable replicas

Documents:
  keep 3 replicas
  one off-site

Temporary media:
  cache only
```

Replica Manager реализует физическую стратегию.

---

# 102. Privacy domains

Space и Entity могут влиять на допустимые вычислительные домены.

Пример:

```text
Work confidential
→ work-managed nodes only

Personal private
→ personal trusted nodes

Public media
→ cloud allowed
```

Это позволяет AI автоматически использовать ресурсы, не нарушая границы данных.

---

# 103. Software trust

Runtime Transparency не должна означать «AI запускает что угодно где угодно».

SoftwareEntity должна иметь trust/provenance:

```text
verified publisher
trusted repository
user supplied
unknown
modified
```

Неизвестный runtime запускается с более строгим sandbox.

---

# 104. Compatibility Knowledge Base

Успешные решения Runtime Resolver должны становиться воспроизводимыми recipes:

```text
software
artifact hash
runtime
dependencies
settings
node constraints
observed result
```

После успешного решения AI не должна каждый раз заново «угадывать».

Knowledge Base должна отделять:

```text
verified recipe
experimental recipe
failed recipe
```

---

# 105. Deterministic before generative

Общий закон SaaiOS:

> Если задача надёжно решается детерминированным механизмом, LLM не должна находиться на критическом пути.

Пример:

```text
known Steam game
+ verified Proton recipe
→ deterministic resolver
```

AI нужна для неизвестных/неструктурированных случаев.

---

# 106. Personal Environment bootstrap

Нужно заранее определить, где находится минимальный authority state, позволяющий восстановить среду.

Вопросы для будущего ADR:

```text
Как восстанавливается User Identity?
Как добавляется первый новый узел?
Какие данные необходимы для восстановления Entity Graph?
Где хранится root trust?
Как восстанавливаются policy?
Что происходит при потере всех текущих devices?
```

Это критическая архитектурная тема PCE.

---

# 107. Local-first, not device-only

SaaiOS должна быть local-first:

```text
ownership local/user controlled
offline capable
cloud optional
```

Но local-first не означает:

```text
everything must execute on the device in hand
```

Домашний доверенный GPU/NAS остаётся local/personal domain даже при удалённом доступе.

---

# 108. Semantic commands over transport commands

Хороший UX:

```text
Распечатай дома.
Открой документ.
Продолжи игру.
Покажи на телевизоре.
Обработай фотографии.
```

Плохой UX как основная модель:

```text
scp
send to device
mount share
remote desktop
choose server
select architecture
```

Транспортные команды остаются в Expert Mode.

---

# 109. Test Network как обязательный архитектурный стенд

Resource Fabric следует проверять на физически разнородной сети как можно раньше.

Минимальный стенд:

```text
Pixel / primary mobile Surface
Linux PC / compute node
small always-on Linux node
storage node
printer or other physical capability
```

Не обязательно иметь мощный GPU для проверки фундаментальной модели.

---

# 110. Killer Demo 1 — Print Anywhere

Начальные условия:

```text
document exists only on Pixel
printer accessible only from Home Node
no manually shared folder
```

Пользователь:

```text
"Распечатай этот документ дома."
```

Успешный flow:

```text
Intent
→ Entity resolve
→ printer capability resolve
→ policy
→ replica/materialization
→ secure transfer
→ print action
→ observation
→ verification
→ "Распечатано"
```

Acceptance criterion:

> Пользователь ни разу не выбирает файл-transfer mechanism, IP, share или intermediate device.

---

# 111. Killer Demo 2 — Open Anywhere

Файл физически существует только на Node A.

На Node B пользователь открывает:

```text
Recent → Document
```

SaaiOS сама:

```text
locates
authenticates
transfers/caches
verifies
opens
```

Acceptance criterion:

> Для пользователя документ является одним объектом, а не набором device-specific copies.

---

# 112. Killer Demo 3 — Compute Anywhere

Тяжёлое действие запускается на Pixel.

Scheduler выбирает более мощный узел.

```text
Pixel
→ Task
→ Home compute
→ result
→ Pixel Surface
```

Затем Home compute выключается.

Следующий Task должен:

```text
fallback
degrade
queue
or select another node
```

без corruption состояния.

---

# 113. Killer Demo 4 — Play Anywhere

Игра доступна в Personal Environment.

Пользователь на Pixel:

```text
Play
```

SaaiOS выбирает Home GPU Node.

```text
execution:
  Home GPU

presentation/input:
  Pixel
```

Позже presentation переносится на TV или desktop без необходимости объяснять пользователю понятие Remote Play.

---

# 114. Killer Demo 5 — Install Without Platform Choice

Пользователь выбирает SoftwareEntity.

SaaiOS должна:

```text
discover compatible artifacts
resolve entitlement
choose runtime
choose node
install
verify
expose Open/Play
```

Пользователь не выбирает:

```text
OS
CPU architecture
compatibility layer
VM
remote execution
```

если нет причины для вмешательства.

---

# 115. Новая структура архитектурных инициатив

К ранее предложенным HIA initiatives добавить:

```text
PCE-01  Device Transparency
PCE-02  Logical Entity Identity
PCE-03  Replica Manager
PCE-04  Content Addressing / Version Model
PCE-05  Personal Fabric Membership
PCE-06  Node Identity / Trust
PCE-07  Capability Advertisement
PCE-08  Capability Leasing
PCE-09  Resource Scheduler
PCE-10  Placement Policy
PCE-11  Data Gravity / Cost Model
PCE-12  Durable Distributed Tasks
PCE-13  Idempotent Actions
PCE-14  Network Partition Semantics
PCE-15  Surface Handoff
PCE-16  Runtime Resolver
PCE-17  Software Entity
PCE-18  Compatibility Knowledge Base
PCE-19  Remote Interactive Execution
PCE-20  Game Streaming Pipeline
PCE-21  Entitlement / Purchase Boundary
PCE-22  Provenance
PCE-23  Backup / Replica Policy
PCE-24  Environment Recovery / Bootstrap
PCE-25  Physical Multi-node Test Lab
```

---

# 116. Definition of Done для PCE-функций

Функция не считается реализованной только потому, что работает happy path.

Для каждого PCE capability проверить:

```text
normal execution
node unavailable before start
node disappears during action
network partition
duplicate request
stale replica
conflicting version
policy denial
insufficient capability
low battery
metered network
executor restart
scheduler restart
device reboot
user cancellation
audit/provenance
recovery
```

---

# 117. Архитектурный тест на правильность абстракции

При проектировании любой функции задавать вопрос:

> «Заставляет ли это пользователя думать о конкретном устройстве, ОС, pathname, runtime или маршруте данных без реальной необходимости?»

Если да — сначала проверить, можно ли перенести эту сложность в SaaiOS.

---

# 118. Обновлённая формула SaaiOS

```text
                            HUMAN
                              │
                             ORB
                              │
                         CONTEXTFRAME
                              │
                ┌─────────────┴─────────────┐
                │                           │
           SPACE GRAPH                 ENTITY GRAPH
                                            │
                                          INTENT
                                            │
                                     WORK SCHEDULER
                                            │
              ┌─────────────────────────────┼─────────────────────────────┐
              │                             │                             │
           POLICY                    CAPABILITY GRAPH               COST / QOS
              │                             │                             │
              └─────────────────────────────┼─────────────────────────────┘
                                            │
                                      RESOURCE FABRIC
                     ┌──────────────────────┼──────────────────────┐
                     │                      │                      │
                  COMPUTE                STORAGE                SENSORS
                     │                      │                      │
                 RUNTIMES                REPLICAS              ACTUATORS
                     │                      │                      │
                     └──────────────────────┼──────────────────────┘
                                            │
                                         SURFACES
                                            │
                                           USER
```

---

# 119. Финальная продуктовая формулировка

> **SaaiOS — это операционная система персональной вычислительной среды. Пользователь взаимодействует со своими объектами и намерениями, а не с границами отдельных устройств. SaaiOS динамически обнаруживает доступные вычисления, данные, сенсоры, исполнительные механизмы и поверхности; применяет policy; выбирает подходящий runtime и место исполнения; перемещает или материализует данные при необходимости; проверяет результат и представляет его там, где это уместно.**

В идеальном случае пользователь перестаёт задаваться вопросами:

```text
Где лежит файл?
На каком устройстве это установлено?
Для какой ОС эта версия?
Как передать это на другой компьютер?
Где запустить вычисление?
Как подключиться к домашнему ПК?
```

и думает только:

```text
Что я хочу сделать?
```

---

# 120. North Star

> **Граница между устройствами должна исчезнуть из повседневного мышления пользователя.**

Не путём сокрытия реальности, а путём создания достаточно сильной системной абстракции над ней.

Устройства остаются физическими носителями возможностей.

ОС становится средой.

Пользователь остаётся центром.
