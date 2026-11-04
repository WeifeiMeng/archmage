import type { ReactNode } from "react";
import { ArrowLeft, Bot, Palette, Share2, UsersRound } from "lucide-react";
import type { AppView } from "./model";
import "./settings.css";

type SettingsSection = Exclude<AppView, "runs">;

type SettingsShellProps = {
  activeView: SettingsSection;
  onNavigate: (view: SettingsSection) => void;
  onBack: () => void;
  children: ReactNode;
};

const sections: Array<{ view: SettingsSection; label: string; icon: typeof Palette }> = [
  { view: "settings", label: "外观", icon: Palette },
  { view: "agents", label: "智能体管理", icon: Bot },
  { view: "capabilities", label: "能力管理", icon: Share2 },
  { view: "teams", label: "团队管理", icon: UsersRound },
];

export function SettingsShell({ activeView, onNavigate, onBack, children }: SettingsShellProps) {
  return <section className="settings-shell">
    <div className="settings-shell-body">
      <nav className="settings-sections" aria-label="设置分类">
        <span className="settings-sidebar-brand">ARCHMAGE</span>
        <p>通用</p>
        {sections.slice(0, 1).map(({ view, label, icon: Icon }) => <button key={view} type="button" className={activeView === view ? "active" : ""} aria-current={activeView === view ? "page" : undefined} onClick={() => onNavigate(view)}><Icon size={17} />{label}</button>)}
        <p>资源管理</p>
        {sections.slice(1).map(({ view, label, icon: Icon }) => <button key={view} type="button" className={activeView === view ? "active" : ""} aria-current={activeView === view ? "page" : undefined} onClick={() => onNavigate(view)}><Icon size={17} />{label}</button>)}
        <button type="button" className="settings-back" onClick={onBack}><ArrowLeft size={17} />返回主界面</button>
      </nav>
      <div className="settings-shell-content">{children}</div>
    </div>
  </section>;
}
