import { Loader2, Save } from "lucide-react";
import type { AssetRecord, AudioVoiceCard, VisualStyleCard } from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import type { AssetCatalogItem, AssetCatalog } from "./studioModel";
import type { SourceFileSummary } from "./tauriBridge";
import { Collapsible, studioUiClassNames } from "./studioUi";

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
  const recordCount = assetCatalog.items.filter(
    (item) => item.source === "record",
  ).length;
  const referenceCount = assetCatalog.items.reduce(
    (total, item) =>
      item.source === "record" ? total + item.record.references.length : total,
    0,
  );

  return (
    <section className={studioUiClassNames.panel}>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">Asset Maintenance</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {assetCatalog.source === "records"
              ? `${recordCount} asset records`
              : `${assetCatalog.items.length} scene background fallbacks`}
          </p>
        </div>
      </div>

      <SectionStatusMessage section="assets" formStatus={formStatus} />

      <div className="mt-4 grid gap-4 xl:grid-cols-[0.75fr_1.25fr]">
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
          <MetricBox label="Source files" value={sourceFiles.length} />
          <MetricBox label="Asset records" value={recordCount} />
          <MetricBox label="References" value={referenceCount} />
          <MetricBox
            label="Visual cards"
            value={visualBible?.style_cards.length ?? 0}
          />
          <MetricBox
            label="Audio cards"
            value={audioBible?.voice_cards.length ?? 0}
          />
        </div>
        <div className="grid gap-3 md:grid-cols-2">
          {assetCatalog.items.length > 0 ? (
            assetCatalog.items.map((item) => (
              <AssetCatalogCard key={assetCatalogItemKey(item)} item={item} />
            ))
          ) : (
            <EmptyPanel label="No asset records or scene background paths found." />
          )}
        </div>
      </div>

      <div className="mt-5 grid gap-4 lg:grid-cols-2">
        <VisualBibleEditor
          visualBible={visualBible}
          saving={saving}
          onUpdateCard={onUpdateVisualStyleCard}
          onSave={onSaveVisualBible}
        />
        <AudioBibleEditor
          audioBible={audioBible}
          saving={saving}
          onUpdateCard={onUpdateAudioVoiceCard}
          onSave={onSaveAudioBible}
        />
      </div>
    </section>
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
  return (
    <section className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="text-sm font-semibold">Visual Bible</h4>
          <p className="mt-1 text-xs font-medium uppercase text-ink/45">
            {visualBible?.style_cards.length ?? 0} style cards
          </p>
        </div>
        <SaveButton
          label="Save Visual Bible"
          saving={saving}
          onClick={onSave}
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
              badge="style"
              className="rounded-md border border-ink/10 bg-white px-3 py-3"
            >
              <div className="mt-3 grid gap-3">
                <TextareaInput
                  label="Prompt"
                  ariaLabel={`Visual style prompt ${index + 1}`}
                  value={card.prompt}
                  onChange={(value) => onUpdateCard(index, { prompt: value })}
                  minHeight="min-h-28"
                />
                <div className="grid gap-3 sm:grid-cols-3">
                  <TextareaInput
                    label="Palette"
                    ariaLabel={`Visual style palette ${index + 1}`}
                    value={listToLines(card.palette)}
                    onChange={(value) =>
                      onUpdateCard(index, { palette: linesToList(value) })
                    }
                    minHeight="min-h-24"
                  />
                  <TextareaInput
                    label="Tags"
                    ariaLabel={`Visual style tags ${index + 1}`}
                    value={listToLines(card.tags)}
                    onChange={(value) =>
                      onUpdateCard(index, { tags: linesToList(value) })
                    }
                    minHeight="min-h-24"
                  />
                  <TextareaInput
                    label="Reference assets"
                    ariaLabel={`Visual style reference asset ids ${index + 1}`}
                    value={listToLines(card.reference_asset_ids)}
                    onChange={(value) =>
                      onUpdateCard(index, {
                        reference_asset_ids: linesToList(value),
                      })
                    }
                    minHeight="min-h-24"
                  />
                </div>
              </div>
            </Collapsible>
          ))
        ) : (
          <p className="text-sm text-ink/55">
            No Visual Bible style cards in project data.
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
  return (
    <section className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="text-sm font-semibold">Audio Bible</h4>
          <p className="mt-1 text-xs font-medium uppercase text-ink/45">
            {audioBible?.voice_cards.length ?? 0} voice cards
          </p>
        </div>
        <SaveButton
          label="Save Audio Bible"
          saving={saving}
          onClick={onSave}
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
              badge="voice"
              className="rounded-md border border-ink/10 bg-white px-3 py-3"
            >
              <div className="mt-3 grid gap-3">
                <TextInput
                  label="Voice"
                  ariaLabel={`Audio voice ${index + 1}`}
                  value={card.voice}
                  onChange={(value) => onUpdateCard(index, { voice: value })}
                />
                <TextareaInput
                  label="Delivery"
                  ariaLabel={`Audio delivery ${index + 1}`}
                  value={card.delivery}
                  onChange={(value) => onUpdateCard(index, { delivery: value })}
                  minHeight="min-h-24"
                />
                <TextareaInput
                  label="Sample text"
                  ariaLabel={`Audio sample text ${index + 1}`}
                  value={card.sample_text ?? ""}
                  onChange={(value) =>
                    onUpdateCard(index, { sample_text: optionalText(value) })
                  }
                  minHeight="min-h-24"
                />
                <div className="grid gap-3 sm:grid-cols-2">
                  <TextareaInput
                    label="Tags"
                    ariaLabel={`Audio tags ${index + 1}`}
                    value={listToLines(card.tags)}
                    onChange={(value) =>
                      onUpdateCard(index, { tags: linesToList(value) })
                    }
                    minHeight="min-h-24"
                  />
                  <TextareaInput
                    label="Reference assets"
                    ariaLabel={`Audio reference asset ids ${index + 1}`}
                    value={listToLines(card.reference_asset_ids)}
                    onChange={(value) =>
                      onUpdateCard(index, {
                        reference_asset_ids: linesToList(value),
                      })
                    }
                    minHeight="min-h-24"
                  />
                </div>
              </div>
            </Collapsible>
          ))
        ) : (
          <p className="text-sm text-ink/55">
            No Audio Bible voice cards in project data.
          </p>
        )}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// AssetCatalogCard
// ---------------------------------------------------------------------------

function AssetCatalogCard({ item }: { item: AssetCatalogItem }) {
  if (item.source === "scene-background-fallback") {
    return (
      <article className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
        <p className="text-xs font-medium uppercase text-ink/45">
          Scene background fallback
        </p>
        <code className="mt-2 block truncate text-sm text-ink/80">
          {item.path}
        </code>
      </article>
    );
  }

  const { record } = item;
  return (
    <article className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase text-ink/45">
            {record.kind} / {record.source}
          </p>
          <h4 className="mt-1 truncate text-sm font-semibold">{record.id}</h4>
        </div>
        {record.provider_metadata?.fallback_used ? (
          <span className="rounded-md border border-signal/30 bg-signal/10 px-2 py-1 text-xs font-semibold text-signal">
            Fallback
          </span>
        ) : null}
      </div>
      <div className="mt-3 grid gap-2 text-sm text-ink/70">
        <AssetField label="Project path" value={record.project_path} code />
        <AssetField label="Export path" value={record.export_path} code />
        <AssetField label="Content hash" value={record.content_hash} code />
        <AssetField label="Hash algorithm" value={record.hash_algorithm} />
        <AssetField label="Bytes" value={String(record.byte_length)} />
        <AssetField label="Provider" value={providerLabel(record)} />
        <AssetField
          label="Request id"
          value={record.provider_metadata?.request_id ?? "none"}
          code={Boolean(record.provider_metadata?.request_id)}
        />
        <AssetField
          label="Prompt hash"
          value={record.provider_metadata?.prompt_hash ?? "none"}
          code={Boolean(record.provider_metadata?.prompt_hash)}
        />
        <AssetField
          label="References"
          value={referenceLabel(record)}
          code={record.references.length > 0}
        />
      </div>
    </article>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to AssetMaintenanceView)
// ---------------------------------------------------------------------------

function SaveButton({
  label,
  saving,
  onClick,
}: {
  label: string;
  saving: boolean;
  onClick(): void;
}) {
  return (
    <button
      type="button"
      disabled={saving}
      onClick={onClick}
      className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
    >
      {saving ? (
        <Loader2 aria-hidden size={16} className="animate-spin" />
      ) : (
        <Save aria-hidden size={16} />
      )}
      {label}
    </button>
  );
}

function MetricBox({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-ink/10 bg-parchment px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/55">{label}</p>
      <p className="mt-1 truncate font-semibold">{value}</p>
    </div>
  );
}

function EmptyPanel({ label }: { label: string }) {
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-parchment px-3 py-2 text-sm text-ink/55">
      {label}
    </div>
  );
}

function SectionStatusMessage({
  section,
  formStatus,
}: {
  section: StudioSectionId;
  formStatus: FormStatus | null;
}) {
  if (!formStatus || formStatus.section !== section) {
    return null;
  }
  const toneClass =
    formStatus.tone === "success"
      ? "border-jade/30 bg-jade/10 text-jade"
      : "border-signal/30 bg-signal/10 text-signal";
  return (
    <div className={`mt-4 rounded-md border px-3 py-2 text-sm ${toneClass}`}>
      {formStatus.message}
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

function TextInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <input
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={studioUiClassNames.input}
      />
    </label>
  );
}

function TextareaInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
  minHeight = "min-h-40",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
  minHeight?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <textarea
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={`${studioUiClassNames.textarea} ${minHeight}`}
      />
    </label>
  );
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

function assetCatalogItemKey(item: AssetCatalogItem): string {
  return item.source === "record" ? item.record.id : item.path;
}

function providerLabel(record: AssetRecord): string {
  const metadata = record.provider_metadata;
  if (!metadata) {
    return "none";
  }
  return [metadata.provider, metadata.model].filter(Boolean).join(" / ");
}

function referenceLabel(record: AssetRecord): string {
  if (record.references.length === 0) {
    return "none";
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
