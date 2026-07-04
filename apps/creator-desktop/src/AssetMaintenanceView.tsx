import type { AssetRecord, AudioVoiceCard, VisualStyleCard } from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import type { AssetCatalogItem, AssetCatalog } from "./assetCatalog";
import type { SourceFileSummary } from "./tauriBridge";
import {
  Collapsible,
  Reveal,
  SaveButton,
  SectionStatusMessage,
  StudioTabs,
  TextInput,
  TextareaInput,
  ViewHeader,
  studioUiClassNames,
} from "./studioUi";
import {
  PaginationControls,
  PaginatedCardGrid,
  usePagination,
} from "./pagination";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};

export interface VisualBibleData {
  style_cards: VisualStyleCard[];
}

export interface AudioBibleData {
  voice_cards: AudioVoiceCard[];
}

export interface AssetMaintenanceViewProps {
  /** Asset catalog produced by projectAssetCatalog(). */
  assetCatalog: AssetCatalog;
  /** Source file list for metrics. */
  sourceFiles: SourceFileSummary[];
  /** Visual Bible document, or null if not loaded. */
  visualBible: VisualBibleData | null;
  /** Audio Bible document, or null if not loaded. */
  audioBible: AudioBibleData | null;
  /** True while either bible is saving. */
  saving: boolean;
  /** Cross-section form status for displaying save/error feedback. */
  formStatus: FormStatus | null;
  /** Callback: update a single style card field by index. */
  onUpdateVisualStyleCard(index: number, patch: Partial<VisualStyleCard>): void;
  /** Callback: update a single voice card field by index. */
  onUpdateAudioVoiceCard(index: number, patch: Partial<AudioVoiceCard>): void;
  /** Callback: save the current Visual Bible to the data source. */
  onSaveVisualBible(): void;
  /** Callback: save the current Audio Bible to the data source. */
  onSaveAudioBible(): void;
}

// ---------------------------------------------------------------------------
// AssetMaintenanceView
// ---------------------------------------------------------------------------

export function AssetMaintenanceView({
  assetCatalog,
  sourceFiles,
  visualBible,
  audioBible,
  saving,
  formStatus,
  onUpdateVisualStyleCard,
  onUpdateAudioVoiceCard,
  onSaveVisualBible,
  onSaveAudioBible,
}: AssetMaintenanceViewProps) {
  const { t } = useStudioI18n();
  const recordCount = assetCatalog.items.filter(
    (item) => item.source === "record",
  ).length;
  const referenceCount = assetCatalog.items.reduce(
    (total, item) =>
      item.source === "record" ? total + item.record.references.length : total,
    0,
  );

  return (
    <Reveal as="section" className={`${studioUiClassNames.panel} !p-4 grid`}>
      <ViewHeader
        title={t("assets.title")}
        subtitle={
          assetCatalog.source === "records"
            ? t("common.assetRecords", { count: recordCount })
            : t("common.sceneBgFallbacks", { count: assetCatalog.items.length })
        }
      />

      <SectionStatusMessage section="assets" formStatus={formStatus} />

      <StudioTabs
        ariaLabel={t("assets.aria.maintenanceSurfaces")}
        className="mt-4"
        items={[
          {
            id: "visual",
            label: t("assets.visualBible"),
            badge: visualBible?.style_cards.length,
            children: (
              <VisualBibleEditor
                visualBible={visualBible}
                saving={saving}
                onUpdateCard={onUpdateVisualStyleCard}
                onSave={onSaveVisualBible}
              />
            ),
          },
          {
            id: "audio",
            label: t("assets.audioBible"),
            badge: audioBible?.voice_cards.length,
            children: (
              <AudioBibleEditor
                audioBible={audioBible}
                saving={saving}
                onUpdateCard={onUpdateAudioVoiceCard}
                onSave={onSaveAudioBible}
              />
            ),
          },
          {
            id: "catalog",
            label: t("assets.assetCatalog"),
            badge: recordCount,
            children: (
              <div className="grid gap-4 lg:grid-cols-[240px_minmax(0,1fr)]">
                <div className="grid content-start gap-3 sm:grid-cols-3 lg:grid-cols-1">
                  <MetricBox label={t("assets.sourceFiles")} value={sourceFiles.length} />
                  <MetricBox label={t("assets.assetRecords")} value={recordCount} />
                  <MetricBox label={t("assets.references")} value={referenceCount} />
                  <MetricBox
                    label={t("assets.visualCards")}
                    value={visualBible?.style_cards.length ?? 0}
                  />
                  <MetricBox
                    label={t("assets.audioCards")}
                    value={audioBible?.voice_cards.length ?? 0}
                  />
                </div>
                <AssetCatalogList items={assetCatalog.items} />
              </div>
            ),
          },
          {
            id: "source-files",
            label: t("assets.sourceFilesTab"),
            badge: sourceFiles.length,
            children: (
              <div className="grid gap-3">
                <p className="text-sm text-ink/55">
                  {t("assets.sourceFilesNote")}
                </p>
                <div className="grid gap-3 xl:grid-cols-2">
                  {sourceFiles.map((file) => (
                    <article
                      key={file.path}
                      className="rounded-lg border border-canvas-200 bg-canvas-50 p-4 text-ink shadow-studio-panel"
                    >
                      <div className="flex flex-wrap items-start justify-between gap-3">
                        <div className="min-w-0">
                          <p className="text-xs font-semibold uppercase text-graphite-700/45">
                            {file.kind}
                          </p>
                          <h4 className="mt-1 truncate text-base font-semibold">
                            {file.path}
                          </h4>
                        </div>
                        <span
                          className={`rounded-sm border px-2 py-1 text-xs font-semibold ${
                            file.editable
                              ? "border-health-500/35 bg-health-500/15 text-health-500"
                              : "border-canvas-200/40 bg-canvas-100/60 text-graphite-700/80"
                          }`}
                        >
                          {file.editable ? t("common.editable") : t("common.readOnly")}
                        </span>
                      </div>
                      <p className="mt-3 text-xs text-ink/55">
                        {t("assets.bytes")}: {file.bytes}
                      </p>
                    </article>
                  ))}
                </div>
              </div>
            ),
          },
        ]}
      />
    </Reveal>
  );
}

// ---------------------------------------------------------------------------
// VisualBibleEditor
// ---------------------------------------------------------------------------

function VisualBibleEditor({
  visualBible,
  saving,
  onUpdateCard,
  onSave,
}: {
  visualBible: VisualBibleData | null;
  saving: boolean;
  onUpdateCard(index: number, patch: Partial<VisualStyleCard>): void;
  onSave(): void;
}) {
  const { t } = useStudioI18n();
  return (
    <section className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="text-sm font-semibold">{t("assets.visualBible")}</h4>
          <p className="mt-1 text-xs font-medium uppercase text-ink/45">
            {t("common.styleCards", { count: visualBible?.style_cards.length ?? 0 })}
          </p>
        </div>
        <SaveButton
          label={t("assets.saveVisualBible")}
          saving={saving}
          onSave={onSave}
          ariaLabel={t("assets.saveVisualBible")}
        />
      </div>
      <div className="mt-3 grid gap-3">
        {visualBible && visualBible.style_cards.length > 0 ? (
          visualBible.style_cards.map((card, index) => (
            <Collapsible
              key={`${card.id}:${index}`}
              label={card.title || card.id}
              id={`visual-card-${card.id}-${index}`}
              defaultOpen={false}
              badge={t("assets.styleBadge")}
              className="transition ease-expo hover:-translate-y-0.5 hover:shadow-panel-lift rounded-md border border-ink/10 bg-canvas-50 px-3 py-3"
            >
              <div className="mt-3 grid gap-3">
                <TextareaInput
                  label={t("assets.prompt")}
                  ariaLabel={t("assets.aria.visualStylePromptN", { index: index + 1 })}
                  value={card.prompt}
                  onChange={(value) => onUpdateCard(index, { prompt: value })}
                   minHeight="min-h-24"
                />
                <div className="grid gap-3 sm:grid-cols-3">
                  <TextareaInput
                    label={t("assets.palette")}
                    ariaLabel={t("assets.aria.visualStylePaletteN", { index: index + 1 })}
                    value={listToLines(card.palette)}
                    onChange={(value) =>
                      onUpdateCard(index, { palette: linesToList(value) })
                    }
                    minHeight="min-h-20"
                  />
                  <TextareaInput
                    label={t("assets.tags")}
                    ariaLabel={t("assets.aria.visualStyleTagsN", { index: index + 1 })}
                    value={listToLines(card.tags)}
                    onChange={(value) =>
                      onUpdateCard(index, { tags: linesToList(value) })
                    }
                    minHeight="min-h-20"
                  />
                  <TextareaInput
                    label={t("assets.referenceAssets")}
                    ariaLabel={t("assets.aria.visualStyleRefN", { index: index + 1 })}
                    value={listToLines(card.reference_asset_ids)}
                    onChange={(value) =>
                      onUpdateCard(index, {
                        reference_asset_ids: linesToList(value),
                      })
                    }
                    minHeight="min-h-20"
                  />
                </div>
              </div>
            </Collapsible>
          ))
        ) : (
          <p className="text-sm text-ink/55">
            {t("assets.noVisualCards")}
          </p>
        )}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// AudioBibleEditor
// ---------------------------------------------------------------------------

function AudioBibleEditor({
  audioBible,
  saving,
  onUpdateCard,
  onSave,
}: {
  audioBible: AudioBibleData | null;
  saving: boolean;
  onUpdateCard(index: number, patch: Partial<AudioVoiceCard>): void;
  onSave(): void;
}) {
  const { t } = useStudioI18n();
  return (
    <section className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="text-sm font-semibold">{t("assets.audioBible")}</h4>
          <p className="mt-1 text-xs font-medium uppercase text-ink/45">
            {t("common.voiceCards", { count: audioBible?.voice_cards.length ?? 0 })}
          </p>
        </div>
        <SaveButton
          label={t("assets.saveAudioBible")}
          saving={saving}
          onSave={onSave}
          ariaLabel={t("assets.saveAudioBible")}
        />
      </div>
      <div className="mt-3 grid gap-3">
        {audioBible && audioBible.voice_cards.length > 0 ? (
          audioBible.voice_cards.map((card, index) => (
            <Collapsible
              key={`${card.id}:${index}`}
              label={card.title || card.id}
              id={`audio-card-${card.id}-${index}`}
              defaultOpen={false}
              badge={t("assets.voiceBadge")}
              className="transition ease-expo hover:-translate-y-0.5 hover:shadow-panel-lift rounded-md border border-ink/10 bg-canvas-50 px-3 py-3"
            >
              <div className="mt-3 grid gap-3">
                <TextInput
                  label={t("assets.voice")}
                  ariaLabel={t("assets.aria.audioVoiceN", { index: index + 1 })}
                  value={card.voice}
                  onChange={(value) => onUpdateCard(index, { voice: value })}
                />
                <TextareaInput
                  label={t("assets.delivery")}
                  ariaLabel={t("assets.aria.audioDeliveryN", { index: index + 1 })}
                  value={card.delivery}
                  onChange={(value) => onUpdateCard(index, { delivery: value })}
                  minHeight="min-h-24"
                />
                <TextareaInput
                  label={t("assets.sampleText")}
                  ariaLabel={t("assets.aria.audioSampleTextN", { index: index + 1 })}
                  value={card.sample_text ?? ""}
                  onChange={(value) =>
                    onUpdateCard(index, { sample_text: optionalText(value) })
                  }
                  minHeight="min-h-24"
                />
                <div className="grid gap-3 sm:grid-cols-2">
                  <TextareaInput
                    label={t("assets.tags")}
                    ariaLabel={t("assets.aria.audioTagsN", { index: index + 1 })}
                    value={listToLines(card.tags)}
                    onChange={(value) =>
                      onUpdateCard(index, { tags: linesToList(value) })
                    }
                    minHeight="min-h-20"
                  />
                  <TextareaInput
                    label={t("assets.referenceAssets")}
                    ariaLabel={t("assets.aria.audioRefN", { index: index + 1 })}
                    value={listToLines(card.reference_asset_ids)}
                    onChange={(value) =>
                      onUpdateCard(index, {
                        reference_asset_ids: linesToList(value),
                      })
                    }
                    minHeight="min-h-20"
                  />
                </div>
              </div>
            </Collapsible>
          ))
        ) : (
          <p className="text-sm text-ink/55">
            {t("assets.noAudioCards")}
          </p>
        )}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// AssetCatalogList — paginated asset catalog (search by id / path).
// ---------------------------------------------------------------------------

function AssetCatalogList({ items }: { items: AssetCatalogItem[] }) {
  const { t } = useStudioI18n();
  const {
    query,
    setQuery,
    page,
    setPage,
    totalPages,
    filteredCount,
    visible,
    needsControls,
  } = usePagination(items, {
    filter: (item, q) => {
      const field =
        item.source === "record"
          ? `${item.record.id} ${item.record.project_path}`
          : item.path;
      return field.toLowerCase().includes(q.toLowerCase());
    },
  });
  return (
    <PaginatedCardGrid
      controls={
        <PaginationControls
          needsControls={needsControls}
          query={query}
          setQuery={setQuery}
          page={page}
          setPage={setPage}
          totalPages={totalPages}
          filteredCount={filteredCount}
          searchAriaLabel={t("pagination.aria.searchAssets")}
          searchPlaceholder={t("pagination.searchAssets")}
        />
      }
    >
      {visible.length > 0 ? (
        visible.map((item) => (
          <AssetCatalogCard key={assetCatalogItemKey(item)} item={item} />
        ))
      ) : (
        <EmptyPanel label={t("assets.noAssetRecords")} />
      )}
    </PaginatedCardGrid>
  );
}

// ---------------------------------------------------------------------------
// AssetCatalogCard
// ---------------------------------------------------------------------------

function AssetCatalogCard({ item }: { item: AssetCatalogItem }) {
  const { t } = useStudioI18n();
  if (item.source === "scene-background-fallback") {
    return (
      <article className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
        <p className="text-xs font-medium uppercase text-ink/45">
          {t("assets.sceneBgFallback")}
        </p>
        <code className="mt-2 block truncate text-sm text-ink/80">
          {item.path}
        </code>
      </article>
    );
  }

  const { record } = item;
  return (
    <article className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase text-ink/45">
            {record.kind} / {record.source}
          </p>
          <h4 className="mt-1 truncate text-sm font-semibold">{record.id}</h4>
        </div>
        {record.provider_metadata?.fallback_used ? (
          <span className="rounded-md border border-signal/30 bg-signal/10 px-2 py-1 text-xs font-semibold text-signal">
            {t("assets.fallbackBadge")}
          </span>
        ) : null}
      </div>
      <div className="mt-3 grid gap-2 text-sm text-ink/70">
        <AssetField label={t("assets.projectPath")} value={record.project_path} code />
        <AssetField label={t("assets.exportPath")} value={record.export_path} code />
        <AssetField label={t("assets.contentHash")} value={record.content_hash} code />
        <AssetField label={t("assets.hashAlgorithm")} value={record.hash_algorithm} />
        <AssetField label={t("assets.bytes")} value={String(record.byte_length)} />
        <AssetField label={t("assets.provider")} value={providerLabel(record, t)} />
        <AssetField
          label={t("assets.requestId")}
          value={record.provider_metadata?.request_id ?? t("common.none")}
          code={Boolean(record.provider_metadata?.request_id)}
        />
        <AssetField
          label={t("assets.promptHash")}
          value={record.provider_metadata?.prompt_hash ?? t("common.none")}
          code={Boolean(record.provider_metadata?.prompt_hash)}
        />
        <AssetField
          label={t("assets.referencesLabel")}
          value={referenceLabel(record, t)}
          code={record.references.length > 0}
        />
      </div>
    </article>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to AssetMaintenanceView)
// ---------------------------------------------------------------------------

function MetricBox({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/55">{label}</p>
      <p className="mt-1 truncate font-semibold">{value}</p>
    </div>
  );
}

function EmptyPanel({ label }: { label: string }) {
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-canvas-50 px-3 py-2 text-sm text-ink/55">
      {label}
    </div>
  );
}

function AssetField({
  label,
  value,
  code = false,
}: {
  label: string;
  value: string;
  code?: boolean;
}) {
  return (
    <div className="min-w-0">
      <p className="text-[11px] font-medium uppercase text-ink/45">{label}</p>
      {code ? (
        <code className="block truncate text-xs text-ink/75">{value}</code>
      ) : (
        <p className="truncate text-xs text-ink/75">{value}</p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

function assetCatalogItemKey(item: AssetCatalogItem): string {
  return item.source === "record" ? item.record.id : item.path;
}

function providerLabel(
  record: AssetRecord,
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  const metadata = record.provider_metadata;
  if (!metadata) {
    return t("common.none");
  }
  return [metadata.provider, metadata.model].filter(Boolean).join(" / ");
}

function referenceLabel(
  record: AssetRecord,
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  if (record.references.length === 0) {
    return t("common.none");
  }
  return record.references
    .map(
      (reference) =>
        `${reference.reference_kind}:${reference.reference_id}:${reference.slot}`,
    )
    .join(", ");
}

function listToLines(values: string[]) {
  return values.join("\n");
}

function linesToList(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}
