import { FileText, ShieldAlert } from "lucide-react";
import { artifactSummary } from "../lib/artifacts";
import { messages, type Language } from "../lib/i18n";
import { shortPath } from "../lib/labels";
import type { HarnessArtifact } from "../types";

interface HarnessTableProps {
  artifacts: HarnessArtifact[];
  language: Language;
  workspacePath: string;
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function HarnessTable({
  artifacts,
  language,
  workspacePath,
  selectedId,
  onSelect,
}: HarnessTableProps) {
  const copy = messages[language];
  if (!artifacts.length) {
    return (
      <div className="empty-state compact">
        <FileText size={24} />
        <strong>{copy.table.emptyTitle}</strong>
        <span>{copy.table.emptyBody}</span>
      </div>
    );
  }

  return (
    <div className="table-shell">
      <table className="artifact-table">
        <thead>
          <tr>
            <th>{copy.table.name}</th>
            <th>{copy.table.kind}</th>
            <th>{copy.table.provider}</th>
            <th>{copy.table.scope}</th>
            <th>{copy.table.status}</th>
          </tr>
        </thead>
        <tbody>
          {artifacts.map((artifact) => (
            <tr
              key={artifact.id}
              className={selectedId === artifact.id ? "selected" : undefined}
              tabIndex={0}
              aria-label={copy.table.inspectAria(artifact.name)}
              onClick={() => onSelect(artifact.id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(artifact.id);
                }
              }}
            >
              <td>
                <div className="artifact-name-cell">
                  <div className="file-mark">
                    {artifact.sensitive ? <ShieldAlert size={15} /> : <FileText size={15} />}
                  </div>
                  <div>
                    <strong>{artifact.name}</strong>
                    <span>{shortPath(artifact.path, workspacePath)}</span>
                    <p>{artifactSummary(artifact, {
                      contentNotLoaded: copy.table.contentNotLoaded,
                      noReadableSummary: copy.table.noReadableSummary,
                    })}</p>
                  </div>
                </div>
              </td>
              <td>{copy.labels.kind[artifact.kind]}</td>
              <td>{copy.labels.provider[artifact.provider]}</td>
              <td>{copy.labels.scope[artifact.scope]}</td>
              <td>
                <span className={`status-pill status-${artifact.resolution}`}>
                  {copy.labels.resolution[artifact.resolution]}
                </span>
                {artifact.counterpartId ? (
                  <span className="status-pill status-drifted diagnostic-pill">
                    {copy.labels.resolution.drifted}
                  </span>
                ) : null}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
