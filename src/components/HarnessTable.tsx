import { CircleCheck, FileText, ShieldAlert } from "lucide-react";
import {
  artifactDiagnostics,
  artifactSummary,
  artifactSummarySource,
} from "../lib/artifacts";
import {
  localizeArtifactDiagnostic,
  messages,
  type Language,
} from "../lib/i18n";
import { shortPath } from "../lib/labels";
import type { HarnessArtifact, HarnessWarning } from "../types";

interface HarnessTableProps {
  artifacts: HarnessArtifact[];
  warnings: HarnessWarning[];
  language: Language;
  workspacePath: string;
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function HarnessTable({
  artifacts,
  warnings,
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
            <th>{copy.table.positionPurpose}</th>
            <th>{copy.table.status}</th>
            <th>{copy.table.diagnostics}</th>
          </tr>
        </thead>
        <tbody>
          {artifacts.map((artifact) => {
            const diagnostics = artifactDiagnostics(artifact, warnings);
            const primaryDiagnostic = diagnostics[0]
              ? localizeArtifactDiagnostic(diagnostics[0], artifact, language)
              : null;
            const purpose = artifactSummary(artifact, {
              byKind: copy.artifact.purposeByKind,
            });
            const purposeSource = artifactSummarySource(artifact) === "declared"
              ? copy.artifact.declaredPurpose
              : copy.artifact.generatedPurpose;

            return <tr
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
                  </div>
                </div>
              </td>
              <td>
                <div className="artifact-purpose-cell">
                  <span className="artifact-position">
                    {copy.labels.provider[artifact.provider]} · {copy.labels.scope[artifact.scope]} · {copy.labels.kind[artifact.kind]}
                  </span>
                  <p>{purpose}</p>
                  <small>{purposeSource}</small>
                </div>
              </td>
              <td>
                <span className={`status-pill status-${artifact.resolution}`}>
                  {copy.labels.resolution[artifact.resolution]}
                </span>
              </td>
              <td>
                {primaryDiagnostic ? (
                  <div className={`table-diagnostic severity-${diagnostics[0].warning.severity}`}>
                    <strong>{copy.artifact.diagnosticCount(diagnostics.length)}</strong>
                    <span>{primaryDiagnostic.title}</span>
                  </div>
                ) : (
                  <div className="table-diagnostic no-findings">
                    <CircleCheck size={14} />
                    <span>{copy.artifact.noDiagnostics}</span>
                  </div>
                )}
              </td>
            </tr>
          })}
        </tbody>
      </table>
    </div>
  );
}
