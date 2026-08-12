import { useMemo } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import { messages, type Language } from "../lib/i18n";
import { counterpartDifferenceCount } from "../lib/artifacts";
import type {
  HarnessArtifact,
  HarnessKind,
  HarnessProvider,
  HarnessScope,
  HarnessSnapshot,
} from "../types";

export interface MapFilter {
  provider?: HarnessProvider;
  kind?: HarnessKind;
  scope?: HarnessScope;
}

interface HarnessMapProps {
  snapshot: HarnessSnapshot;
  language: Language;
  onFilter: (filter: MapFilter) => void;
}

type MapNodeData = {
  label: React.ReactNode;
  filter?: MapFilter;
  tone: "root" | "provider" | "kind";
};

function nodeStyle(tone: MapNodeData["tone"]): React.CSSProperties {
  const common: React.CSSProperties = {
    borderRadius: 14,
    border: "1px solid rgba(255,255,255,.11)",
    color: "#edf2f7",
    padding: 0,
    boxShadow: "0 14px 36px rgba(0,0,0,.22)",
    overflow: "hidden",
  };
  if (tone === "root") {
    return { ...common, background: "linear-gradient(135deg,#1a4f46,#12342f)", width: 200 };
  }
  if (tone === "provider") {
    return { ...common, background: "linear-gradient(135deg,#202833,#171d25)", width: 190 };
  }
  return { ...common, background: "#151b22", width: 210 };
}

function label(title: string, subtitle: string, badge?: string) {
  return (
    <div className="map-node-content">
      <div className="map-node-title-row">
        <strong>{title}</strong>
        {badge ? <span className="map-node-badge">{badge}</span> : null}
      </div>
      <span>{subtitle}</span>
    </div>
  );
}

function groupByProvider(artifacts: HarnessArtifact[]) {
  return artifacts.reduce<Record<HarnessProvider, HarnessArtifact[]>>(
    (groups, artifact) => {
      groups[artifact.provider].push(artifact);
      return groups;
    },
    { codex: [], claude: [], shared: [], plugin: [] },
  );
}

export function HarnessMap({ snapshot, language, onFilter }: HarnessMapProps) {
  const copy = messages[language];
  const { nodes, edges } = useMemo(() => {
    const nextNodes: Node<MapNodeData>[] = [];
    const nextEdges: Edge[] = [];
    const groups = groupByProvider(snapshot.artifacts);
    const activeProviders = (Object.keys(groups) as HarnessProvider[]).filter(
      (provider) => groups[provider].length > 0,
    );

    nextNodes.push({
      id: "workspace",
      position: { x: 20, y: Math.max(100, activeProviders.length * 105) },
      sourcePosition: Position.Right,
      data: {
        tone: "root",
        label: label(snapshot.workspaceName, copy.map.harnessItems(snapshot.artifacts.length), snapshot.gitBranch ?? copy.map.local),
      },
      style: nodeStyle("root"),
    });

    let kindRow = 0;
    activeProviders.forEach((provider, providerIndex) => {
      const providerArtifacts = groups[provider];
      const providerNodeId = `provider:${provider}`;
      const providerY = providerIndex * 210 + 24;
      const differenceCount = counterpartDifferenceCount(snapshot, provider);
      nextNodes.push({
        id: providerNodeId,
        position: { x: 300, y: providerY },
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        data: {
          tone: "provider",
          filter: { provider },
          label: label(
            copy.labels.provider[provider],
            copy.map.discovered(providerArtifacts.length),
            differenceCount ? copy.map.drift(differenceCount) : undefined,
          ),
        },
        style: nodeStyle("provider"),
      });
      nextEdges.push({
        id: `workspace-${provider}`,
        source: "workspace",
        target: providerNodeId,
        type: "smoothstep",
        markerEnd: { type: MarkerType.ArrowClosed, color: "#51606f" },
        style: { stroke: "#3d4a57", strokeWidth: 1.5 },
      });

      const kinds = Array.from(new Set(providerArtifacts.map((item) => item.kind))).sort();
      kinds.forEach((kind) => {
        const items = providerArtifacts.filter((item) => item.kind === kind);
        const effectiveCount = items.filter((item) => item.resolution === "effective").length;
        const kindNodeId = `kind:${provider}:${kind}`;
        nextNodes.push({
          id: kindNodeId,
          position: { x: 620, y: kindRow * 92 },
          targetPosition: Position.Left,
          data: {
            tone: "kind",
            filter: { provider, kind },
            label: label(
              copy.labels.kind[kind],
              copy.map.items(items.length),
              effectiveCount ? copy.map.effective(effectiveCount) : undefined,
            ),
          },
          style: nodeStyle("kind"),
        });
        nextEdges.push({
          id: `${providerNodeId}-${kind}`,
          source: providerNodeId,
          target: kindNodeId,
          type: "smoothstep",
          style: { stroke: "#35414d", strokeWidth: 1.2 },
        });
        kindRow += 1;
      });
    });

    return { nodes: nextNodes, edges: nextEdges };
  }, [copy, snapshot]);

  return (
    <div className="map-shell" aria-label={copy.map.ariaLabel}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        fitViewOptions={{ padding: 0.18 }}
        minZoom={0.25}
        maxZoom={1.4}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        onNodeClick={(_, node) => {
          const filter = (node.data as MapNodeData).filter;
          if (filter) onFilter(filter);
        }}
        proOptions={{ hideAttribution: true }}
      >
        <Background variant={BackgroundVariant.Dots} gap={22} size={1} color="#2a333d" />
        <MiniMap
          pannable
          zoomable
          nodeColor={(node) => {
            const tone = (node.data as MapNodeData).tone;
            return tone === "root" ? "#2e8f7e" : tone === "provider" ? "#485869" : "#25303b";
          }}
          maskColor="rgba(8,11,15,.72)"
        />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
