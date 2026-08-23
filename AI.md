# c2pa_view — AI Context

## Overview

**InReality layering:** product-specific provenance visualization (synthetic nodes,
verdict highlights, origin badges, graph windowing) lives in the separate
`inreality_c2pa_view` package and attaches via a generic annotation sidecar —
not by extending `ProvenanceNode`. Canonical three-layer boundaries, join keys,
and OSS vs private ownership: `fleet/decisions/d0042-inreality-c2pa-viewer-layering.md`.
Verdict evaluation and ruleset `evaluate()` stay **outside** this OSS package
(in `inreality_c2pa_view` / ruleset bridge per [`d0043`](../../fleet/decisions/d0043-client-authoritative-verdict-display.md)).

`c2pa_view` is an open-source Flutter plugin that reads and displays [C2PA](https://c2pa.org/) (Coalition for Content Provenance and Authenticity) content credentials. It extracts embedded C2PA manifests from media files using the official `c2pa-rs` Rust library via Flutter Rust Bridge, then renders the provenance data as interactive Flutter widgets.

**Package name:** `c2pa_view`
**Version:** 0.4.0
**Homepage:** https://github.com/in-reality/c2pa_view
**Platforms:** Android, iOS, Linux, macOS, Windows (FFI plugin)

## Architecture

### Data Flow

```
File / URL / Bytes
       ↓
  Rust FFI (c2pa-rs)  →  JSON string with validation_status
       ↓
  ManifestStore.fromJson()  →  Domain entities (Equatable)
       ↓  (two display paths)
       ├── ProvenanceMapper.mapToGraph()  →  ProvenanceGraph
       │       ↓
       │   C2paManifestViewer          (full view: tree + detail panel)
       │
       └── ManifestViewDataMapper.map()  →  ManifestViewData
               ↓
           showManifestDetailPopup()    (popup overlay from a button)
```

### Layer Structure

```
lib/
├── c2pa_view.dart                      # Barrel export (public API)
├── api.dart                            # Legacy free-function API
├── core/
│   ├── bridge/c2pa_bridge_service.dart # Static wrapper: FFI → ManifestStore
│   └── theme/c2pa_theme.dart           # C2paViewerThemeData + InheritedWidget
├── domain/
│   ├── entities/                       # Raw C2PA JSON → Equatable domain models
│   ├── models/                         # Display-ready view models
│   └── mappers/                        # Entity → ViewModel transformations
├── features/
│   ├── manifest_viewer/                # C2paManifestViewer (composite widget)
│   ├── provenance_tree/                # Zoomable/pannable DAG viewer
│   ├── manifest_detail/                # Detail panel, popup, content sections
│   ├── custom_fields/                  # Vendor-specific assertion table
│   └── shared/widgets/                 # Reusable cards, indicators, sections
└── src/rust/                           # Generated flutter_rust_bridge bindings
```

### Rust Layer

The `rust/` directory contains a Cargo crate (`cdylib` + `staticlib` + `lib`) that wraps `c2pa-rs`. Eight async FFI functions are exposed on the Rust worker pool (no sync wire for parse/validate):

| Function | Input | Output |
|---|---|---|
| `get_file_manifest` | bytes + file path | manifest JSON (MIME guessed from path) |
| `get_file_manifest_format` | bytes + MIME type | manifest JSON |
| `get_manifest_with_validation` | bytes + MIME type | manifest JSON with `validation_status` injected |
| `get_manifest_with_validation_from_path` | file path only | manifest JSON with `validation_status` injected |
| `get_manifest_with_trust_validation` | bytes + MIME + trust PEM | trust-validated manifest JSON |
| `get_manifest_with_trust_validation_from_path` | path + trust PEM | trust-validated manifest JSON |
| `get_*_utf8` variants | bytes + format | UTF-8 JSON bytes (web-safe large returns) |

Path-based entry points read the file on the Rust side; byte-based entry points remain for callers that only have in-memory bytes. The Dart-side `C2paBridgeService` and `ManifestStore.fromLocalPath` / `fromBytes` are async and await the bridge.

### Domain Entities

All entities use `Equatable` for value equality.

| Entity | C2PA Concept |
|---|---|
| `ManifestStore` | Root: contains all manifests, the active manifest label, and top-level validation |
| `Manifest` | Single manifest with assertions, signature, ingredients, actions |
| `Ingredient` | Source file reference (may link to another manifest) |
| `Action` | Recorded editing/creation action (e.g. `c2pa.created`, `c2pa.edited`) |
| `SignatureInfo` / `CertificateInfo` | Signing certificate chain data |
| `ExifData` | EXIF metadata (camera, lens, exposure, GPS) |
| `CreativeWork` / `SocialAccount` | schema.org CreativeWork assertion |
| `TrainingMining` | AI training/mining preference (`c2pa.training-mining`) |
| `ThumbnailData` | Embedded thumbnail (raw bytes or data URI) |
| `CustomField` | Generic key-value for vendor-specific assertions |
| `ValidationStatusEntry` | Individual validation check result (code + URL + explanation) |

### View Models

| Model | Purpose |
|---|---|
| `ManifestViewData` | All display-ready data for one manifest's detail view |
| `ManifestSummary` | Compact summary (title, thumbnail, validation status, issuer) |
| `ProvenanceGraph` / `ProvenanceNode` / `ProvenanceEdge` | DAG structure for the tree viewer |
| `ValidationResult` / `ValidationStatus` | Normalized validation state (valid / invalid / untrusted / noCredential) |
| `GenerativeInfo` / `GenerativeType` | AI generation detection |

### Widgets

| Widget | Description |
|---|---|
| `C2paManifestViewer` | Full composite: provenance tree (left) + detail panel (right) |
| `ProvenanceTreeViewer` | Zoomable, pannable DAG with interactive node selection |
| `ManifestDetailPanel` | Fixed-width sidebar showing manifest details |
| `ManifestDetailContent` | Scrollable detail body (used by both panel and popup) |
| `showManifestDetailPopup()` | Anchored popup overlay showing manifest details |
| `CustomFieldsTable` | Flattened key-value table for vendor-specific fields |

### Theme

`C2paViewerThemeData` is propagated via `C2paViewerTheme` (an `InheritedWidget`). It controls:
- Validation status colors (valid, invalid, untrusted, no-credential)
- Surface/border/text/icon colors
- Six text style tiers (titleLarge through label)
- Layout dimensions (sidebar width, thumbnail size, node dimensions, spacing)
- Border radii

A `C2paViewerThemeData.dark()` factory provides dark-mode defaults.

## Dependencies

| Dependency | Role |
|---|---|
| `flutter_rust_bridge` 2.12.0 | Rust ↔ Dart FFI bridge |
| `c2pa-rs` (fork: NorthGuard/c2pa-rs) | C2PA manifest reading and validation |
| `equatable` | Value equality for domain entities |
| `flutter_svg` | Renders the Content Credentials "CR" pin asset |
| `http` | URL fetching for `fromUrl` |
| `intl` | Date formatting in detail views |

## Initialization

Before any C2PA operations, the Rust library must be initialized:

```dart
await RustLib.init();
```

This loads the native binary (or WASM on web). Must be called once, typically in `main()` after `WidgetsFlutterBinding.ensureInitialized()`.

## Key Patterns

- **Entities are immutable.** All domain entities extend `Equatable` and have `const` constructors.
- **Mappers are pure static classes.** `ProvenanceMapper` and `ManifestViewDataMapper` contain only static methods with no side effects.
- **Two display paths.** The full viewer (`C2paManifestViewer`) needs a `ProvenanceGraph`; the popup (`showManifestDetailPopup`) needs a `ManifestViewData`. Both are derived from the same `ManifestStore`.
- **Theme is opt-in.** Widgets fall back to `C2paViewerThemeData.defaults` when no `C2paViewerTheme` ancestor exists.
- **Generated code in `src/rust/`.** Never hand-edit files under `lib/src/rust/` — run `flutter_rust_bridge_codegen generate` after modifying `rust/src/api/*.rs`.

## Extension API

Downstream packages decorate provenance graphs via a parallel annotation
sidecar — `ProvenanceNode` / `ProvenanceEdge` stay pure C2PA. Empty
annotations preserve the default appearance. Canonical layering:
`fleet/decisions/d0042-inreality-c2pa-viewer-layering.md`.

### Data types

| Type | Purpose |
|---|---|
| `ProvenanceAnnotations` | Sidecar with `nodeDecorations`, `edgeDecorations`, `detailSections`, `attachments` maps |
| `NodeDecoration` / `EdgeDecoration` | Opaque host slots (border hints, badges, payload) |
| `NodeAttachment` | Satellite box anchored outside layout (`id`, `anchorNodeId`, `color`, `payload`) |
| `DecorationBadge` | Compact badge on a node (`id`, `label`, optional `icon`, `payload`) |
| `GraphHighlight` / `HighlightStyle` | Stacked highlight layers per node or edge |
| `GraphHighlights` | Resolved maps keyed by node id and `provenanceEdgeId(parent, child)` |
| `ManifestDetailSection` | Host detail-panel section (`id`, `order`, `builder`) |
| `ProvenanceInteractionHandler` | Outbound-only tap/focus callbacks |

Join keys: node maps use `ProvenanceNode.id` (manifest label); edge maps use
`provenanceEdgeId(parentId, childId)` → `"$parentId→$childId"`.

### Widget parameters

| Widget | Extension params |
|---|---|
| `ProvenanceTreeViewer` | `annotations`, `highlights`, `nodeDecorator`, `edgeDecorator`, `interactionHandler`, `attachmentContentBuilder` |
| `C2paManifestViewer` | Forwards the same params; passes `detailSections` for the selected node |
| `ManifestDetailContent` / `ManifestDetailPanel` | `extraSections` |

`ProvenanceNodeDecorator` wraps the default node card; `ProvenanceEdgeDecorator`
runs after the default edge stroke. Selection-path highlighting is expressed as
built-in highlight layers (`C2paHighlightLayers`) merged with host-supplied
`GraphHighlights`, resolved from the inherited `C2paViewerTheme` at paint time.

`C2paManifestViewer` wraps a non-null `interactionHandler` with
`ComposingProvenanceInteractionHandler` so detail-panel selection stays in sync
before the host handler runs. Badge taps with an `icon` route to `onIconTap`;
label-only badges use `onBadgeTap`. Attachment taps route to `onAttachmentTap`.
Satellite attachments fan around anchor nodes in graph space, are excluded from
depth/row layout, and are included in fit-to-view bounds. Fan spacing is tuned
for up to eight satellites per anchor; larger companion sets (for example after
content-`data_hash` collapse) use a denser semicircle — all remain visible, but
crowding may warrant a grouping / "+N" overflow affordance tracked in the parent
repo's `fleet/initiatives/i0026-media-node-satellite-manifests.md`. Invalid
anchors are never re-homed: debug builds assert, release builds omit attachments
whose `anchorNodeId` is not a rendered graph node or does not match the
attachments-map key. `onEdgeTap` and
`onFocusChangeRequested` are part of the exported contract but are not yet
invoked by `ProvenanceTreeViewer` (edge hit-testing deferred).

### Theme slots

`C2paViewerThemeData` adds generic decoration colors:

- `highlightAccentColor` — default accent for badges and unresolved highlights
- `highlightFillColor` — optional node fill tint
- `highlightBadgeTextColor` — badge label color

## Testing

- `testfiles_app/` is a standalone Flutter app that loads C2PA test files from URLs and demonstrates both the full viewer and the popup.
- `testfiles_app/integration_test/` contains integration tests for manifest parsing.
- `rust/tests/validate_evidence.rs` contains Rust-level tests validating positive files, negative (tampered) files, and files with no manifest.
