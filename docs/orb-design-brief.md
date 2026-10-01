# SaaiOS Orb

## Design Brief & Interaction Specification

This document is the product specification for Orb as the primary spatial navigation interface of SaaiOS. It is written for a designer. It states the philosophy, the mechanics, and what must not be designed.

It is not the Stage presence indicator in [Platform 0.6](platform-0.6-gui.md). That indicator is a small health affordance on the conversation surface. This Orb is the geography of the environment.

### 1. Purpose

Orb is the primary spatial navigation interface of SaaiOS.

It is **not**:

- an application launcher;
- a radial menu;
- a floating AI button;
- a decorative 3D sphere;
- a replacement for application windows;
- a carousel of shortcuts;
- a traditional desktop rendered on a sphere.

Orb is a continuous spatial interface for navigating the capabilities, objects, services, devices, applications, tasks, and contexts available in the user's current SaaiOS environment.

Its purpose is to replace the traditional concept of:

- desktops;
- pages of icons;
- application folders;
- nested menus;
- multiple home screens;
- separate device-control dashboards.

The central design idea is:

> The screen is finite. The environment does not have to be.

Orb creates a virtually unlimited navigational space by projecting a much larger spatial structure through the limited viewport of a physical display.

The user never needs to see the whole structure at once.

This is similar conceptually to navigating Earth through a map or globe: only a fraction of the total surface is visible, yet the entire structure remains navigable.

### 2. Fundamental mental model

Imagine a sphere substantially larger than the portion visible on screen.

Most of this sphere may exist beyond or below the physical viewport.

The user sees only a small portion of its surface.

Objects belonging to SaaiOS exist at positions within this navigational space.

The user can:

- rotate the Orb;
- move across its surface;
- zoom in;
- zoom out;
- change semantic depth;
- navigate between echelons;
- search for an object;
- jump directly to an object;
- follow a visual route to an object;
- activate an object or capability.

The sphere therefore represents a **navigation space**, not a literal physical object.

The visual representation may be 3D, 2.5D, projected, simplified, or adapted to device capabilities.

The spatial model is more important than the rendering technique.

### 3. Resting state

Orb should always feel like part of the environment rather than an application that has to be opened.

In its resting state, most of the Orb is below the visible surface of the interface.

The user sees only a small exposed portion or interaction point.

Conceptually:

```text
        SCREEN / ENVIRONMENT
────────────────────────────────────

              •
           Orb point
              │
        .-------------.
     .-'               '-.
   .'                     '.
  /                         \
 /                           \
        MOST OF ORB
      BELOW SURFACE
```

The Orb should not permanently consume significant screen space.

It exists as a persistent affordance.

It should communicate:

> "There is more environment here."

without demanding attention.

### 4. Activation

When the user taps, clicks, focuses, or otherwise activates the Orb:

1. the sphere rises slightly from beneath the surface;
2. the currently relevant echelon becomes visible;
3. nearby capabilities appear on or around the visible surface;
4. the user can immediately rotate or navigate it.

The movement should feel physical and continuous.

Do not present a separate "Orb screen."

The user should feel that the same object that was partially hidden has simply become available for interaction.

There should be no conceptual transition from:

"desktop → menu."

Instead:

"environment → deeper interaction with environment."

### 5. Continuous surface

Orb has no conventional first or last page.

Do not design:

```text
Page 1 → Page 2 → Page 3 → Page 4
```

Design:

```text
             continuous
          navigational space

                ↗ ↑ ↖
              ←  ●  →
                ↘ ↓ ↙
```

The user can continue moving through the space.

Objects outside the viewport still exist.

This allows the interface to scale from ten capabilities to potentially thousands without requiring additional home screens.

The visible screen becomes a viewport into a larger information space.

### 6. Echelons

Orb supports multiple **echelons**.

An echelon is not simply a submenu.

It represents another organizational layer of the environment.

The first echelon should contain capabilities that are common across most contexts.

Examples might include:

- Search;
- Sai;
- current tasks;
- applications;
- notifications;
- system;
- recent objects.

These examples are illustrative, not mandatory.

Other echelons may be generated from the current environment.

Examples:

```text
ECHELON 1
Universal capabilities

ECHELON 2
Current environment

ECHELON 3
Context-specific capabilities

ECHELON N
Specialized / deep objects
```

The number of echelons is not inherently fixed.

The designer should avoid visual metaphors that impose an artificial maximum.

### 7. Semantic zoom

Zoom is not merely graphical scaling.

It changes **semantic resolution**.

Consider:

```text
HOME
 ↓ zoom
Rooms / systems
 ↓ zoom
Workshop
 ↓ zoom
Devices
 ↓ zoom
3D printer
 ↓ zoom
Capabilities
 ↓ zoom
Print / temperature / queue / materials
```

At a distant zoom level, individual low-level controls should disappear.

At closer levels, additional structure becomes visible.

This is similar to map systems:

```text
Planet
Country
City
Street
Building
Object
```

Orb applies the same principle to an operating environment.

Examples:

```text
Work
 └── SaaiOS
      ├── Builds
      ├── Devices
      ├── Repository
      └── Tasks
```

or:

```text
Home
 └── Climate
      ├── Living room
      ├── Bedroom
      └── Workshop
```

The designer should explore ways of making semantic zoom understandable without showing conventional directory trees.

### 8. Spatial memory

Spatial consistency is critical.

Users should gradually learn:

> "That thing is over there."

Frequently used or established objects should not randomly change position.

Dynamic context may affect:

- visibility;
- prominence;
- availability;
- suggested objects;
- temporary overlays.

It should not constantly destroy spatial memory.

The system may suggest better placement, but stable user-learned geography has priority.

Think of Orb as having geography.

Not necessarily literal continents, but recognizable regions, directions, clusters, landmarks, and neighborhoods.

### 9. Search

Search is an integral navigation mechanism.

Traditional operating systems typically behave like this:

```text
search
  ↓
result
  ↓
open
```

Orb should support:

```text
search
  ↓
find object in spatial environment
  ↓
show direction / route
  ↓
optionally navigate toward it
  ↓
user learns its location
```

A search result should therefore be capable of doing two things:

#### Direct action

Open or activate the result immediately.

#### Spatial guidance

Show where the object exists within Orb.

Potential visual mechanisms:

- direction indicator;
- arc;
- highlighted horizon;
- trail;
- temporary navigation path;
- controlled rotation toward the result;
- zoom guidance.

The exact mechanism is a design problem.

The important principle is:

> Search should help users learn the environment instead of permanently replacing navigation.

### 10. Dynamic environment

Orb represents the **currently available environment**.

The environment may change based on:

- device;
- location/context supplied by SaaiOS;
- connected hardware;
- available services;
- user permissions;
- network availability;
- current task;
- active project;
- installed applications;
- remote SaaiOS nodes.

Example:

At home, the environment might expose:

```text
Music
Lighting
Climate
Cameras
Media
Computers
Tasks
```

In a workshop:

```text
Projects
Tools
3D printer
Electronics
Measurements
Documentation
```

On a laptop:

```text
Browser
Files
Development
Music
Games
Remote devices
```

These are not separate Orb implementations.

They are different projections of the same interface model.

### 11. Capabilities, not applications

A crucial design principle:

**Orb primarily exposes what the user can do, not which executable provides it.**

For example:

The user may see:

```text
Music
```

rather than:

```text
Spotify
VLC
MPD
Bluetooth Speaker
Browser
```

When appropriate, SaaiOS chooses or offers the implementation.

Similarly:

```text
Scan document
```

may internally use:

- a phone camera;
- a network scanner;
- an existing application;
- another SaaiOS device.

The implementation is secondary to the capability.

Applications still exist and remain accessible.

Orb simply does not require the user to think in applications for every action.

### 12. Objects are heterogeneous

Orb must not assume every object is an application icon.

An object may represent:

- application;
- action;
- task;
- person;
- device;
- service;
- document;
- project;
- environment;
- media;
- automation;
- AI agent;
- remote computer;
- sensor;
- capability;
- notification;
- system state.

The visual language must therefore support multiple object classes without becoming a collection of unrelated icon styles.

### 13. Cross-device universality

The same conceptual Orb should work across very different devices.

#### Phone

Primary input:

- touch;
- swipe;
- pinch;
- tap.

#### Laptop / desktop

Primary input:

- mouse;
- wheel;
- keyboard;
- touchpad;
- optional touch.

#### Gaming handheld / Steam Deck

Primary input:

- analog sticks;
- trackpads;
- triggers;
- buttons.

#### Large display

May support:

- pointer;
- touch;
- remote controller.

#### Low-power hardware

May render a simplified projection.

The navigation model must remain recognizable across all of them.

Do not design the Orb specifically around smartphone gestures.

### 14. The wheel principle

An important interaction inspiration comes from early Sony mobile devices that used a physical side wheel for menu navigation.

The important lesson is not nostalgia or visual imitation.

The useful property was:

> A large navigational space could be explored continuously with an extremely small and intuitive physical action.

Orb should preserve this quality.

Examples:

```text
Mouse wheel      → navigate / rotate
Touch swipe      → navigate / rotate
Analog stick     → navigate / rotate
Trackpad         → navigate / rotate
Physical wheel   → navigate / rotate
```

Different hardware inputs should map onto the same conceptual motion.

Interaction should become muscle memory.

### 15. Depth without requiring literal 3D

Orb uses spatial depth conceptually.

The user does not need stereoscopic vision, VR, or a fully rendered globe.

The physical display remains two-dimensional.

The system creates perceived depth through:

- projection;
- occlusion;
- scaling;
- motion;
- semantic zoom;
- perspective;
- layering;
- horizon behavior;
- object density.

The goal is not:

> "Look at our 3D interface."

The goal is:

> "This environment feels larger than the screen."

### 16. Applications and windows still exist

Orb should not attempt to replace interfaces that already work well.

A browser remains a browser.

A text editor remains a text editor.

A game remains a game.

A terminal remains a terminal.

Orb replaces the **organization layer above them**.

Traditional model:

```text
Desktop
 ├── icons
 ├── folders
 ├── launcher
 ├── virtual desktops
 ├── taskbar
 └── windows
```

SaaiOS model:

```text
Environment
      │
     Orb
      │
objects / capabilities / tasks / devices
      │
application or action when needed
```

Applications can still open conventional windows.

The user simply no longer needs hundreds of windows and virtual desktops as the primary method of organizing digital life.

### 17. Sai and Orb

Sai may interact with Orb but does not own it.

Orb must work without AI.

Sai may:

- find objects;
- suggest capabilities;
- explain an object;
- navigate to something;
- propose organization;
- surface relevant actions;
- create temporary contextual clusters.

Sai must not:

- invent unavailable capabilities;
- silently rearrange established user geography;
- make navigation dependent on model availability.

Orb belongs to SaaiOS.

Sai is an intelligent participant in the environment.

### 18. Calm interface

Orb should avoid becoming visually noisy.

Do not fill the sphere with hundreds of simultaneously visible icons.

Use semantic zoom, prioritization and occlusion.

At any moment, only a manageable subset of information should demand attention.

The visual hierarchy should communicate:

```text
important now
available nearby
available deeper
exists but currently irrelevant
```

The user should feel that the environment is large but calm.

Not empty.

Not crowded.

### 19. Dynamic but predictable

This is one of the hardest design requirements.

Orb is dynamic because the environment changes.

Yet it must remain predictable enough to build muscle and spatial memory.

Therefore distinguish between:

#### Persistent geography

User-known locations and major categories.

#### Contextual objects

Appear when relevant.

#### Temporary suggestions

May disappear.

#### Availability state

Existing object becomes unavailable without ceasing to exist.

Example:

If a home server goes offline, its object should not simply vanish.

It may become dimmed or otherwise indicate:

```text
known
currently unavailable
```

Existence and availability are different concepts.

### 20. Open architecture

Orb must not require modifications whenever SaaiOS gains a new capability.

New components should be able to describe themselves through a common object/capability model.

Conceptually:

```text
Capability
    ↓
SaaiOS environment model
    ↓
Orb projection
    ↓
appropriate representation
```

Therefore the visual system should be capable of rendering previously unknown object types through generic primitives.

Special representations may exist, but they must not be required for basic functionality.

### 21. Thin integration philosophy

SaaiOS follows a principle of **Thin Integration**.

Existing software should be reused whenever practical.

Orb therefore should not visually imply that every capability is a native SaaiOS application.

A capability might be implemented by:

- Linux application;
- web service;
- Android application;
- Windows compatibility layer;
- remote device;
- open-source daemon;
- hardware controller;
- Sai-generated automation.

The user interacts with the capability.

SaaiOS handles the implementation boundary.

### 22. Visual personality

The interface should communicate:

- intelligence without anthropomorphic gimmicks;
- technological sophistication without cyberpunk clutter;
- depth without excessive 3D effects;
- calmness;
- precision;
- safety;
- continuity;
- responsiveness.

Avoid:

- neon overload;
- HUD aesthetics;
- excessive glassmorphism;
- holographic sci-fi clichés;
- rotating decorative particles;
- permanently animated backgrounds;
- giant AI avatars;
- visual complexity for its own sake.

Orb should look like something a person could comfortably use for years.

### 23. Motion

Motion communicates geometry.

Animation should therefore explain:

- where an object came from;
- where it went;
- how the sphere rotated;
- whether the user moved or changed echelon;
- whether semantic depth changed.

Avoid arbitrary transitions.

If the user rotates the Orb and selects an object, its transition into the resulting interface should preserve spatial continuity where practical.

Motion should reinforce the mental model.

### 24. Performance requirement

Interaction must feel immediate.

Orb cannot depend on remote AI inference for:

- rotation;
- zoom;
- object selection;
- navigation;
- search through locally indexed objects;
- echelon changes.

These operations belong to the local UI/runtime.

AI-enhanced operations may happen asynchronously.

The interface must remain usable when Sai is unavailable.

### 25. Accessibility

The spatial interface must not become mandatory.

SaaiOS should be capable of exposing the same environment through alternative representations such as:

- search;
- list;
- keyboard navigation;
- screen-reader-friendly hierarchy;
- direct commands.

Orb is the primary visual navigation model, not the only possible accessibility path.

### 26. Design challenge

The designer should answer:

#### Geometry

How much of the sphere is visible when resting and active?

#### Horizon

How do objects enter and leave the visible region?

#### Density

How many objects can remain visible without clutter?

#### Semantic zoom

How does an object transform as more detail becomes available?

#### Echelons

How does the user understand that another organizational layer exists without introducing conventional tabs?

#### Navigation

How do touch, mouse wheel, trackpad and controller produce equivalent navigation?

#### Search guidance

How does search show where an object lives?

#### Context changes

How does Orb communicate that the surrounding environment changed?

#### Stability

How are persistent objects protected from unwanted rearrangement?

#### Availability

How does an unavailable but known object differ from something that no longer exists?

### 27. Prototype scenarios

Do not begin by drawing isolated screens.

Prototype interaction sequences.

#### Scenario A — resting → application

1. Orb mostly hidden.
2. User activates it.
3. Orb rises.
4. User rotates it.
5. Browser becomes visible.
6. User selects browser.
7. Browser opens.
8. Orb recedes.

#### Scenario B — semantic zoom

1. User sees `Home`.
2. Zooms toward it.
3. Home systems appear.
4. Selects `Climate`.
5. Rooms appear.
6. Selects workshop.
7. Current temperature and climate actions appear.

#### Scenario C — search

1. User searches for `3D printer`.
2. Search finds the object.
3. Orb indicates its direction.
4. User chooses "show me."
5. Orb rotates and changes semantic depth.
6. Printer appears highlighted.
7. User now knows approximately where it lives.

#### Scenario D — context change

1. User leaves home.
2. Home capabilities remain known but remote.
3. Phone-local and mobile capabilities gain relevance.
4. Fundamental Orb geography does not collapse or randomly rearrange.

#### Scenario E — another device

Repeat Scenario A on:

- phone;
- laptop;
- Steam Deck.

The user should recognize the same interaction model immediately.

### 28. Success criterion

A successful Orb design should make the following statement believable:

> "I don't navigate applications. I navigate my digital environment."

The user should eventually stop thinking about:

- which home screen contains an icon;
- which virtual desktop contains a window;
- which application controls a device;
- which computer contains a capability.

Instead, they should develop an intuitive understanding of where things exist within their environment.

### 29. Core design principle

Do not attempt to display the whole system.

That would reproduce the problem Orb is intended to solve.

The fundamental advantage of the concept is that:

> **Only the locally relevant part of an effectively unlimited environment needs to be visible at any moment.**

The screen is a viewport.

Orb is the geography.

SaaiOS provides the environment.

Sai helps the user understand and manipulate it.
