import { teamRunOverview, type Team } from "./model";
import "./runTeam.css";

export function RunTeamPreview({ team }: { team: Team }) {
  const overview = teamRunOverview(team);
  return <section className="run-team-preview" aria-label="团队组成">
    <div className="run-team-preview-heading"><strong>团队组成</strong><span>{team.graph.mode === "flow" ? "消息流" : "历史咨询"}</span></div>
    <div className="run-team-members">{overview.members.map((member) => <span key={member}>{member}</span>)}</div>
    <p>{overview.detail}</p>
    <div className="run-team-routes" aria-label="团队关系">{overview.routes.map((route) => <span key={route}>{route}</span>)}</div>
  </section>;
}
