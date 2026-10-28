import type { ReactNode } from "react";
import "./management.css";

type ManagementHeroProps = {
  eyebrow: string;
  title: string;
  description?: string;
  actions?: ReactNode;
  summary?: ReactNode;
  children?: ReactNode;
};

export type PageMetricTone = "default" | "ready" | "warning" | "failed";

export function ManagementHero({ eyebrow, title, description, actions, summary, children }: ManagementHeroProps) {
  return <header className="management-hero">
    <div className="management-hero-top">
      <div className="management-hero-copy">
        <p className="eyebrow">{eyebrow}</p>
        <h1>{title}</h1>
        {description && <p>{description}</p>}
      </div>
      {actions && <div className="management-hero-actions">{actions}</div>}
    </div>
    {summary && <div className="management-hero-summary">{summary}</div>}
    {children}
  </header>;
}

export function PageMetric({ label, value, detail, tone = "default" }: { label: string; value: ReactNode; detail?: string; tone?: PageMetricTone }) {
  return <div className={`page-metric ${tone}`}>
    <span>{label}</span>
    <strong>{value}</strong>
    {detail && <small>{detail}</small>}
  </div>;
}
