import { FolderOpen, RefreshCw, Square } from "lucide-react";
import "./RunHeaderActions.css";

export function RunHeaderActions({ workspaceAvailable, cancellable, onRefresh, onReveal, onCancel }: {
  workspaceAvailable: boolean;
  cancellable: boolean;
  onRefresh: () => void;
  onReveal: () => void;
  onCancel: () => void;
}) {
  return <div className="header-actions" role="group" aria-label="Run 操作">
    <button onClick={onRefresh}><RefreshCw size={16} />刷新</button>
    <button onClick={onReveal} disabled={!workspaceAvailable}><FolderOpen size={16} />工作区</button>
    {cancellable && <button className="danger" onClick={onCancel}><Square size={14} fill="currentColor" />取消</button>}
  </div>;
}
