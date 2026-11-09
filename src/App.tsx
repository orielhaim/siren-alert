import { useEffect, useState, useRef } from "react";
import { getVersion } from "@tauri-apps/api/app";
import {
  PiGithubLogo as Github,
  PiDownloadSimple as Download,
} from "react-icons/pi";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  PiBellRinging as BellRing,
  PiBroadcast as Radio,
  PiMonitor as Monitor,
  PiPlay as Play,
  PiX as X,
  PiKey as KeyRound,
  PiSpeakerHigh as Volume2,
  PiArrowRight as ArrowRight,
  PiArrowsClockwise as RefreshCw,
  PiArrowLeft as ArrowLeft,
  PiPlus as Plus,
  PiTimer as Timer,
  PiTranslate as Languages,
  PiArrowSquareOut as ExternalLink,
} from "react-icons/pi";
import "./App.css";
import { AppleSwitch } from "./components/ui/switch";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "./components/ui/select";
import WarmTooltip from "./components/WarmTooltip";
import { TooltipButton } from "./components/TooltipButton";
import { SoundPicker } from "./components/SoundPicker";
import {
  MultiSelect,
  MultiSelectTrigger,
  MultiSelectValue,
  MultiSelectInput,
  MultiSelectContent,
  MultiSelectList,
  MultiSelectItem,
  MultiSelectEmpty,
} from "./components/ui/multi-select";
import { settingsWriter } from "./lib/settings-writer";
import { SplashBackground } from "./components/SplashBackground";
import { useTranslation } from "react-i18next";
import type { Language } from "./i18n";
type Style = "corner_left" | "corner_right" | "center" | "fullscreen";
type Settings = {
  style: Style;
  cities: string[];
  sound: boolean;
  warningSound: string;
  alertSound: string;
  countdown: boolean;
  language: Language;
};
type City = {
  name: string;
  translations: Record<string, string>;
  countdown: number | null;
};
type Sound = { id: string; name: string; custom: boolean };
type Alert = {
  id: string;
  type: string;
  cities: string[];
  title: string | null;
  instructions: string | null;
  isTest: boolean;
  receivedAt: string | null;
};
type View = {
  settings: Settings;
  status: string;
  alerts: Alert[];
  hasKey: boolean;
  autostart: boolean;
};
const styles: { value: Style; name: string }[] = [
  { value: "corner_left", name: "Top left" },
  { value: "corner_right", name: "Top right" },
  { value: "center", name: "Centered" },
  { value: "fullscreen", name: "Full screen" },
];
const initial: View = {
  settings: {
    style: "corner_right",
    cities: [],
    sound: true,
    warningSound: "disabled",
    alertSound: "siren1",
    countdown: true,
    language: "en",
  },
  status: "Loading…",
  alerts: [],
  hasKey: false,
  autostart: false,
};
const alertWindow = new URLSearchParams(location.search).has("alert");
function App() {
  const [view, setView] = useState(initial);
  const [loaded, setLoaded] = useState(false);
  const [editingKey, setEditingKey] = useState(false);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [key, setKey] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [saveState, setSaveState] = useState<
    "idle" | "saving" | "saved" | "error"
  >("idle");
  const latest = useRef<Settings | null>(null);
  const writer = useRef(
    settingsWriter<Settings>((value) =>
      invoke("save_settings", { settings: value }),
    ),
  );
  const [update, setUpdate] = useState("");
  const [version, setVersion] = useState("");
  useEffect(() => {
    void getVersion()
      .then(setVersion)
      .catch(() => {});
  }, []);
  const [cities, setCities] = useState<City[]>([]);
  const [sounds, setSounds] = useState<Sound[]>([]);
  const [previewing, setPreviewing] = useState("");
  const [countdowns, setCountdowns] = useState<Record<string, number>>({});
  const [now, setNow] = useState(Date.now());
  const settings = draft ?? view.settings;
  const { t, i18n } = useTranslation();
  const language: Language = i18n.resolvedLanguage === "he" ? "he" : "en";
  useEffect(() => {
    if (loaded) void i18n.changeLanguage(settings.language);
  }, [loaded, settings.language, i18n]);
  const displayStatus = () => {
    switch (view.status) {
      case "Starting":
        return t("status.starting");
      case "Connecting":
        return t("status.connecting");
      case "Syncing current alerts":
        return t("status.syncing");
      case "Disconnected. Retrying automatically":
        return t("status.disconnected");
      case "Connection timed out":
        return t("status.timeout");
      case "Snapshot unavailable":
        return t("status.snapshotUnavailable");
      default:
        return view.status;
    }
  };
  const styleName = (style: Style) =>
    ({
      corner_left: t("left"),
      corner_right: t("right"),
      center: t("center"),
      fullscreen: t("full"),
    })[style];
  useEffect(() => {
    document.documentElement.lang = language;
    document.documentElement.dir = i18n.dir(language);
  }, [language, i18n]);
  useEffect(() => {
    if (!alertWindow) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  useEffect(() => {
    let disposed = false;
    const events = [
      listen<string>("audio-error", (e) => setError(e.payload)),
      listen<boolean>("audio-playing", (e) => {
        if (!e.payload) setPreviewing("");
      }),
      listen("catalog-updated", () => {
        void invoke<Record<string, number>>("get_countdowns").then((v) => {
          if (!disposed) setCountdowns(v);
        });
      }),
    ];
    void invoke<Record<string, number>>("get_countdowns").then((v) => {
      if (!disposed) setCountdowns(v);
    });
    return () => {
      disposed = true;
      for (const event of events) void event.then((fn) => fn());
    };
  }, []);
  useEffect(() => {
    let disposed = false;
    const subscription = listen<View>("state", ({ payload }) => {
      if (!disposed) setView(payload);
    });
    subscription
      .then(() => invoke<View>("get_state"))
      .then((v) => {
        if (!disposed) {
          setView(v);
          setLoaded(true);
        }
      })
      .catch((e) => {
        if (!disposed) setError(String(e));
      });
    return () => {
      disposed = true;
      void subscription.then((unlisten) => unlisten());
    };
  }, []);
  useEffect(() => {
    if (alertWindow || !view.hasKey) return;
    invoke<City[]>("list_cities")
      .then(setCities)
      .catch((e) => setError(String(e)));
  }, [view.hasKey]);
  useEffect(() => {
    if (alertWindow || !view.hasKey) return;
    void invoke<Sound[]>("list_sounds")
      .then(setSounds)
      .catch((e) => setError(String(e)));
  }, [view.hasKey]);
  useEffect(() => {
    if (!alertWindow) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void invoke("dismiss");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  function updateSettings(next: Settings) {
    latest.current = next;
    setDraft(next);
    setSaveState("saving");
    setError("");
    void writer.current
      .write(next)
      .then(async () => {
        if (latest.current !== next) return;
        const current = await invoke<View>("get_state");
        if (latest.current !== next) return;
        setView(current);
        setDraft(null);
        setSaveState("saved");
      })
      .catch((e) => {
        if (latest.current === next) {
          setSaveState("error");
          setError(String(e));
        }
      });
  }
  async function action(work: () => Promise<unknown>) {
    setBusy(true);
    setError("");

    try {
      await work();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  if (alertWindow)
    return (
      <main
        className={`alert-surface ${view.alerts.every((a) => a.type === "endAlert") ? "ended" : ""}`}
        dir="rtl"
      >
        <div className="alert-top">
          <span>
            <Radio size={16} /> SIREN ALERT
          </span>
          <button
            aria-label={t("dismiss")}
            onClick={() => void invoke("dismiss")}
          >
            <X />
          </button>
        </div>
        <div className="alert-content" role="alert">
          {view.alerts.some((a) => a.isTest) && (
            <strong className="test-label">{t("testOnly")}</strong>
          )}
          <BellRing size={42} />
          {view.alerts.map((a) => (
            <article key={a.id}>
              <h1>{a.title || a.type}</h1>
              <div className="alert-cities">
                {a.cities.map((city) => {
                  const seconds = countdowns[city];
                  const timestamp = a.receivedAt
                    ? Date.parse(a.receivedAt)
                    : NaN;
                  const remaining =
                    seconds === undefined ||
                    !Number.isFinite(timestamp) ||
                    a.type === "newsFlash" ||
                    a.type === "endAlert"
                      ? null
                      : Math.max(
                          0,
                          Math.ceil((timestamp + seconds * 1000 - now) / 1000),
                        );
                  return (
                    <div className="alert-city" key={city}>
                      <span>{city}</span>
                      {view.settings.countdown && remaining !== null && (
                        <time aria-label={t("seconds", { count: remaining })}>
                          <Timer size={14} />
                          {remaining > 0
                            ? `${Math.floor(remaining / 60)}:${String(remaining % 60).padStart(2, "0")}`
                            : t("shelter")}
                        </time>
                      )}
                    </div>
                  );
                })}
              </div>
              {a.instructions && <p>{a.instructions}</p>}
            </article>
          ))}
          {error && <p>{error}</p>}
        </div>
        <footer>{t("alertFooter")}</footer>
      </main>
    );
  if (!loaded)
    return (
      <main className="splash-screen" role="status">
        <SplashBackground />
        <div className="splash-brand">
          <Radio size={46} strokeWidth={1.4} />
          <h1>Siren Alert</h1>
          <div className="boot-progress" />
          <p>{error || t("ready")}</p>
        </div>
      </main>
    );
  if (!view.hasKey || editingKey)
    return (
      <main className="login-screen">
        <SplashBackground />
        <TooltipButton
          className="language-login icon-button"
          title={t("language")}
          aria-label={t("language")}
          onClick={() =>
            void i18n.changeLanguage(language === "en" ? "he" : "en")
          }
        >
          <Languages size={17} />
          {language === "en" ? "עברית" : "English"}
        </TooltipButton>
        {view.hasKey && (
          <TooltipButton
            className="back-button icon-button"
            aria-label={t("back")}
            title={t("back")}
            onClick={() => {
              setEditingKey(false);
              setKey("");
              setError("");
            }}
          >
            <ArrowLeft size={18} />
          </TooltipButton>
        )}
        <form
          className="login-form"
          onSubmit={(e) => {
            e.preventDefault();
            void action(async () => {
              await invoke("set_key", { key });
              setKey("");
              const next = await invoke<View>("get_state");
              const preferences = { ...next.settings, language };
              await invoke("save_settings", { settings: preferences });
              setView({ ...next, settings: preferences });
              setEditingKey(false);
            });
          }}
        >
          <Radio className="login-mark" size={42} strokeWidth={1.5} />
          <h1>Siren Alert</h1>
          <p className="login-hint">{t("keyHint")}</p>
          <input
            aria-label={t("key")}
            type="password"
            autoComplete="off"
            autoFocus
            placeholder={t("key")}
            value={key}
            disabled={busy}
            onChange={(e) => setKey(e.target.value)}
          />
          <button
            className="primary login-submit"
            disabled={busy || !key.trim()}
          >
            {busy ? t("connecting") : t("connect")}
            <ArrowRight size={16} />
          </button>
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
          <div className="onboarding-links">
            <button
              type="button"
              onClick={() =>
                void action(() => invoke("open_help", { portal: true }))
              }
            >
              {t("portal")}
              <ExternalLink size={12} />
            </button>
            <button
              type="button"
              onClick={() =>
                void action(() => invoke("open_help", { portal: false }))
              }
            >
              {t("help")}
              <ExternalLink size={12} />
            </button>
          </div>
        </form>
      </main>
    );
  return (
    <main className="settings">
      <header className="app-header">
        <div className="brand">
          <Radio size={22} />
          <h1>Siren Alert</h1>
        </div>
        <WarmTooltip
          content={displayStatus()}
          surfaceColor="#353130"
          inkColor="#f2e9e5"
          size="sm"
        >
          <span
            className={`connection ${view.status === "Connected" ? "online" : ""}`}
            role="status"
          >
            <i />
            {view.status === "Connected" ? t("connected") : displayStatus()}
          </span>
        </WarmTooltip>
      </header>
      <div
        className="style-options"
        role="radiogroup"
        aria-label={t("position")}
      >
        {styles.map((style, index) => (
          <WarmTooltip
            key={style.value}
            content={styleName(style.value)}
            className="placement-tip"
            surfaceColor="#353130"
            inkColor="#f2e9e5"
            size="sm"
          >
            <button
              key={style.value}
              role="radio"
              aria-label={styleName(style.value)}
              aria-checked={settings.style === style.value}
              tabIndex={settings.style === style.value ? 0 : -1}
              className={`style-option ${settings.style === style.value ? "selected" : ""}`}
              onKeyDown={(e) => {
                if (
                  ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(
                    e.key,
                  )
                ) {
                  e.preventDefault();
                  const step =
                    e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : -1;
                  const next =
                    styles[(index + step + styles.length) % styles.length];
                  updateSettings({ ...settings, style: next.value });
                  const target = e.currentTarget
                    .closest('[role="radiogroup"]')
                    ?.querySelectorAll('[role="radio"]')
                    .item((index + step + styles.length) % styles.length);
                  if (target instanceof HTMLElement) target.focus();
                }
              }}
              onClick={() => {
                updateSettings({ ...settings, style: style.value });
              }}
            >
              <div className={`mini-screen ${style.value}`} aria-hidden="true">
                <span className="mini-alert">
                  <i />
                  <b />
                  <em />
                </span>
              </div>
            </button>
          </WarmTooltip>
        ))}
      </div>
      <div className="locations">
        <div className="city-picker">
          <MultiSelect
            value={settings.cities}
            onValueChange={(cities) => updateSettings({ ...settings, cities })}
          >
            <MultiSelectTrigger className="city-trigger">
              <MultiSelectValue
                placeholder={t("all")}
                removeLabel={(name) => t("removeCity", { name })}
              />
              <MultiSelectInput
                aria-label={t("search")}
                placeholder={settings.cities.length ? t("search") : ""}
              />
            </MultiSelectTrigger>
            <MultiSelectContent className="city-options">
              <MultiSelectList ariaLabel={t("search")}>
                {cities.map((city) => (
                  <MultiSelectItem
                    value={city.name}
                    textValue={city.translations[language] || city.name}
                    keywords={Object.values(city.translations)}
                    key={city.name}
                  >
                    {city.translations[language] || city.name}
                  </MultiSelectItem>
                ))}
                <MultiSelectEmpty>{t("empty")}</MultiSelectEmpty>
              </MultiSelectList>
            </MultiSelectContent>
          </MultiSelect>{" "}
          <TooltipButton
            className="icon-button"
            title={t("refresh")}
            aria-label={t("refresh")}
            disabled={busy}
            onClick={() =>
              void action(async () =>
                setCities(await invoke<City[]>("refresh_cities")),
              )
            }
          >
            <RefreshCw size={15} />
          </TooltipButton>
        </div>
      </div>
      <div className="preferences">
        <div className="setting-row">
          <Volume2 size={17} />
          <span>{t("sound")}</span>
          <AppleSwitch
            className="settings-switch"
            size="sm"
            tone="accent"
            aria-label={t("sound")}
            checked={settings.sound}
            onCheckedChange={(sound) => updateSettings({ ...settings, sound })}
          />
        </div>
        <div className="sound-slots">
          <SoundPicker
            value={settings.warningSound}
            sounds={sounds}
            label={t("warning")}
            disabled={!settings.sound || busy}
            onConfirm={(id) =>
              updateSettings({ ...settings, warningSound: id })
            }
            onError={setError}
          />
          <SoundPicker
            value={settings.alertSound}
            sounds={sounds}
            label={t("alert")}
            disabled={!settings.sound || busy}
            onConfirm={(id) => updateSettings({ ...settings, alertSound: id })}
            onError={setError}
          />
        </div>
        <div className="sound-library">
          <button
            className="text-button"
            disabled={busy}
            onClick={() =>
              void action(async () => {
                const imported = await invoke<Sound | null>("import_sound");
                if (imported) setSounds(await invoke<Sound[]>("list_sounds"));
              })
            }
          >
            <Plus size={14} />
            {t("add")}
          </button>
          {sounds.some((sound) => sound.custom) && (
            <details>
              <summary>
                {t("custom")} · {sounds.filter((s) => s.custom).length}
              </summary>
              {sounds
                .filter((sound) => sound.custom)
                .map((sound) => (
                  <div className="library-item" key={sound.id}>
                    <input
                      aria-label={`${t("rename")} ${sound.name}`}
                      defaultValue={sound.name}
                      maxLength={100}
                      onBlur={(e) => {
                        const name = e.target.value.trim();
                        if (name && name !== sound.name)
                          void action(async () => {
                            await invoke("rename_sound", {
                              id: sound.id,
                              name,
                            });
                            setSounds(await invoke<Sound[]>("list_sounds"));
                          });
                      }}
                    />
                    <TooltipButton
                      className="icon-button"
                      aria-label={`${t("preview")} ${sound.name}`}
                      onClick={() =>
                        void action(async () => {
                          if (previewing === sound.id) {
                            await invoke("stop_sound");
                            setPreviewing("");
                          } else {
                            await invoke("preview_sound", { id: sound.id });
                            setPreviewing(sound.id);
                          }
                        })
                      }
                    >
                      <Play size={13} />
                    </TooltipButton>
                    <TooltipButton
                      className="icon-button"
                      aria-label={`${t("remove")} ${sound.name}`}
                      disabled={
                        busy ||
                        settings.warningSound === sound.id ||
                        settings.alertSound === sound.id
                      }
                      title={
                        settings.warningSound === sound.id ||
                        settings.alertSound === sound.id
                          ? t("removeAssigned")
                          : t("remove")
                      }
                      onClick={() =>
                        void action(async () => {
                          await invoke("remove_sound", { id: sound.id });
                          setSounds(await invoke<Sound[]>("list_sounds"));
                        })
                      }
                    >
                      <X size={13} />
                    </TooltipButton>
                  </div>
                ))}
            </details>
          )}
        </div>
        <div className="setting-row">
          <Timer size={17} />
          <span>{t("countdown")}</span>
          <AppleSwitch
            className="settings-switch"
            size="sm"
            tone="accent"
            aria-label={t("countdown")}
            checked={settings.countdown}
            onCheckedChange={(countdown) =>
              updateSettings({ ...settings, countdown })
            }
          />
        </div>
        <div className="setting-row">
          <Monitor size={17} />
          <span>{t("startup")}</span>
          <AppleSwitch
            className="settings-switch"
            size="sm"
            tone="accent"
            aria-label={t("startup")}
            checked={view.autostart}
            disabled={busy}
            onCheckedChange={(enabled) =>
              void action(() => invoke("set_autostart", { enabled }))
            }
          />
        </div>
      </div>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="actions">
        <button
          className="secondary"
          disabled={busy || saveState === "saving" || saveState === "error"}
          onClick={() =>
            void action(async () => {
              await writer.current.settled();
              await invoke("preview_alert");
            })
          }
        >
          <Play size={14} />
          {t("test")}
        </button>
        <span className="save-status" role="status">
          {saveState === "saving"
            ? t("saving")
            : saveState === "saved"
              ? t("saved")
              : ""}
        </span>
        {saveState === "error" && (
          <button
            className="text-button"
            onClick={() => updateSettings(latest.current || settings)}
          >
            {t("retry")}
          </button>
        )}
      </div>
      <footer className="app-footer">
        <div className="footer-project">
          <span>v{version}</span>
          <TooltipButton
            className="repository-link"
            aria-label="GitHub · orielhaim/siren-alert"
            title="orielhaim/siren-alert"
            onClick={() => void action(() => invoke("open_repository"))}
          >
            <Github size={17} />
            <span>GitHub</span>
          </TooltipButton>
        </div>
        <div>
          <Select
            className="language-picker"
            value={language}
            onValueChange={(value) =>
              updateSettings({
                ...settings,
                language: value === "he" ? "he" : "en",
              })
            }
          >
            <SelectTrigger
              className="language-trigger"
              aria-label={t("language")}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="language-options">
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="he">עברית</SelectItem>
            </SelectContent>
          </Select>

          <TooltipButton
            className="icon-button"
            aria-label={t("changeKey")}
            title={t("changeKey")}
            onClick={() => {
              setEditingKey(true);
              setError("");
            }}
          >
            <KeyRound size={15} />
          </TooltipButton>
          <TooltipButton
            className="icon-button"
            aria-label={t("updates")}
            title={t("updates")}
            disabled={busy}
            onClick={() =>
              void action(async () =>
                setUpdate(
                  await invoke<string>("check_update", { install: false }),
                ),
              )
            }
          >
            <Download size={15} />
            <span>{t("updates")}</span>
          </TooltipButton>
        </div>
        <a
          className="siren-credit"
          href="https://siren.co.il"
          target="_blank"
          rel="noopener noreferrer"
          onClick={(event) => {
            event.preventDefault();
            void invoke("open_help", { portal: true }).catch((error) =>
              setError(String(error)),
            );
          }}
        >
          <img src="/siren-logo.png" width="18" height="18" alt="" />
          <span>{t("poweredBy")}</span>
        </a>
      </footer>
      {update && (
        <div className="update-message" role="status">
          {update}
          {update.startsWith("Update ") && (
            <button
              className="secondary"
              disabled={busy}
              onClick={() =>
                void action(() => invoke("check_update", { install: true }))
              }
            >
              {t("install")}
            </button>
          )}
        </div>
      )}
    </main>
  );
}
export default App;
