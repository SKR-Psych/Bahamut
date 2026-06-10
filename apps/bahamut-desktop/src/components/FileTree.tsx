import { useState } from "react";
import type { FileNode } from "../lib/types";

interface FileTreeProps {
  nodes: FileNode[];
  selectedPath: string | null;
  /** Fired for every row click (files and folders) — drives explorer actions. */
  onSelect: (node: FileNode) => void;
  onOpenFile: (node: FileNode) => void;
}

function TreeNode({
  node,
  depth,
  selectedPath,
  onSelect,
  onOpenFile,
}: {
  node: FileNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (node: FileNode) => void;
  onOpenFile: (node: FileNode) => void;
}) {
  const [expanded, setExpanded] = useState(depth === 0);
  const isSelected = selectedPath === node.path;

  if (node.is_dir) {
    return (
      <div>
        <button
          type="button"
          className={`tree-row tree-dir${isSelected ? " tree-selected" : ""}`}
          style={{ paddingLeft: `${depth * 14 + 8}px` }}
          onClick={() => {
            setExpanded(!expanded);
            onSelect(node);
          }}
          aria-expanded={expanded}
        >
          <span className="tree-caret">{expanded ? "▾" : "▸"}</span>
          {node.name}
        </button>
        {expanded &&
          (node.children ?? []).map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              onOpenFile={onOpenFile}
            />
          ))}
      </div>
    );
  }

  return (
    <button
      type="button"
      className={`tree-row tree-file${isSelected ? " tree-selected" : ""}`}
      style={{ paddingLeft: `${depth * 14 + 24}px` }}
      onClick={() => {
        onSelect(node);
        onOpenFile(node);
      }}
    >
      {node.name}
    </button>
  );
}

export function FileTree({ nodes, selectedPath, onSelect, onOpenFile }: FileTreeProps) {
  if (nodes.length === 0) {
    return <p className="tree-empty">No displayable files in this folder.</p>;
  }
  return (
    <div className="file-tree" role="tree">
      {nodes.map((node) => (
        <TreeNode
          key={node.path}
          node={node}
          depth={0}
          selectedPath={selectedPath}
          onSelect={onSelect}
          onOpenFile={onOpenFile}
        />
      ))}
    </div>
  );
}
