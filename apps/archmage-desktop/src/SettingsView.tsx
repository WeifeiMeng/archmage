import { Check, Moon, Sun } from "lucide-react";
import { ManagementHero } from "./ManagementHero";
import type { Theme } from "./theme";
import "./settings.css";

type SettingsViewProps = {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
};

const themes: Array<{ value: Theme; label: string; description: string; icon: typeof Sun }> = [
  { value: "light", label: "浅色", description: "柔和自然的浅灰绿界面", icon: Sun },
  { value: "dark", label: "暗色", description: "适合低光环境的深色界面", icon: Moon },
];

export function SettingsView({ theme, onThemeChange }: SettingsViewProps) {
  return <section className="settings-view">
    <ManagementHero eyebrow="SETTINGS" title="设置" description="调整 Archmage 的使用体验。" />
    <section className="settings-card" aria-labelledby="appearance-heading">
      <div className="settings-card-heading">
        <div><h2 id="appearance-heading">外观</h2><p>选择适合当前环境的界面主题。</p></div>
      </div>
      <div className="theme-options">
        {themes.map(({ value, label, description, icon: Icon }) => {
          const selected = theme === value;
          return <button key={value} type="button" className={selected ? "theme-option selected" : "theme-option"} aria-pressed={selected} onClick={() => onThemeChange(value)}>
            <div className={`theme-preview ${value}`} aria-hidden="true"><span className="theme-preview-sidebar" /><span className="theme-preview-content"><i /><i /><i /></span></div>
            <span className="theme-option-copy"><span className="theme-option-title"><Icon size={17} /><strong>{label}</strong></span><small>{description}</small></span>
            <span className="theme-option-state">{selected ? <><Check size={15} />已选择</> : "选择"}</span>
          </button>;
        })}
      </div>
    </section>
  </section>;
}
