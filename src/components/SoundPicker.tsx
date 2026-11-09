import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  PiCheck as Check,
  PiPlay as Play,
  PiStop as Square,
  PiX as X,
} from "react-icons/pi";
import {
  Combobox,
  ComboboxTrigger,
  ComboboxInput,
  ComboboxContent,
  ComboboxList,
  ComboboxItem,
  ComboboxEmpty,
} from "./ui/combobox";
import { TooltipButton } from "./TooltipButton";
import { useTranslation } from "react-i18next";
type Sound = { id: string; name: string; custom: boolean };
export function SoundPicker({
  value,
  sounds,
  label,
  disabled,
  onConfirm,
  onError,
}: {
  value: string;
  sounds: Sound[];
  label: string;
  disabled: boolean;
  onConfirm: (id: string) => void;
  onError: (error: string) => void;
}) {
  const { t } = useTranslation();
  const [candidate, setCandidate] = useState(value);
  const [playing, setPlaying] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => setCandidate(value), [value]);
  useEffect(() => {
    const event = listen<boolean>("audio-playing", (e) => {
      if (!e.payload) setPlaying(null);
    });
    return () => {
      void event.then((off) => off());
    };
  }, []);
  const stop = () => {
    if (playing) void invoke("stop_sound");
    setPlaying(null);
  };
  async function preview(id = candidate) {
    setBusy(true);
    try {
      if (playing === id) {
        await invoke("stop_sound");
        setPlaying(null);
      } else {
        await invoke("preview_sound", { id });
        setPlaying(id);
      }
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="sound-slot">
      <span>{label}</span>
      <div className="sound-control">
        <Combobox
          value={candidate}
          onValueChange={(id) => {
            stop();
            setCandidate(id);
          }}
          disabled={disabled}
        >
          <ComboboxTrigger className="sound-trigger">
            <ComboboxInput aria-label={label} placeholder={t("searchSounds")} />
            <span
              className="sound-control-actions"
              data-combobox-control-action=""
              onPointerDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
              }}
              onClick={(event) => event.stopPropagation()}
            >
              {" "}
              <TooltipButton
                className="icon-button"
                aria-label={playing === candidate ? t("stop") : t("preview")}
                title={playing === candidate ? t("stop") : t("preview")}
                disabled={disabled || busy || candidate === "disabled"}
                onClick={() => void preview()}
              >
                {playing === candidate ? (
                  <Square size={14} />
                ) : (
                  <Play size={14} />
                )}
              </TooltipButton>
              {candidate !== value && (
                <>
                  <TooltipButton
                    className="icon-button confirm-sound"
                    aria-label={t("useSound")}
                    title={t("useSound")}
                    disabled={disabled || busy}
                    onClick={() => {
                      stop();
                      onConfirm(candidate);
                    }}
                  >
                    <Check size={15} />
                  </TooltipButton>
                  <TooltipButton
                    className="icon-button"
                    aria-label={t("cancel")}
                    title={t("cancel")}
                    onClick={() => {
                      stop();
                      setCandidate(value);
                    }}
                  >
                    <X size={14} />
                  </TooltipButton>
                </>
              )}
            </span>
          </ComboboxTrigger>
          <ComboboxContent className="sound-options">
            <ComboboxList ariaLabel={label}>
              <ComboboxItem value="disabled" textValue={t("disabled")}>
                {t("disabled")}
              </ComboboxItem>
              {sounds.map((source) => {
                const sound = source.custom
                  ? source
                  : {
                      ...source,
                      name: t("soundName", {
                        number: source.id.replace("siren", ""),
                      }),
                    };
                return (
                  <ComboboxItem
                    value={sound.id}
                    textValue={sound.name}
                    key={sound.id}
                    action={
                      <TooltipButton
                        className="icon-button item-preview"
                        aria-label={`${playing === sound.id ? t("stop") : t("preview")} ${sound.name}`}
                        title={`${playing === sound.id ? t("stop") : t("preview")} ${sound.name}`}
                        disabled={disabled || busy}
                        onClick={() => void preview(sound.id)}
                      >
                        {playing === sound.id ? (
                          <Square size={13} />
                        ) : (
                          <Play size={13} />
                        )}
                      </TooltipButton>
                    }
                  >
                    {sound.name}
                  </ComboboxItem>
                );
              })}
              <ComboboxEmpty>{t("noSounds")}</ComboboxEmpty>
            </ComboboxList>
          </ComboboxContent>
        </Combobox>
      </div>
    </div>
  );
}
