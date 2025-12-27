import { useMemo } from 'react'
import ForceGraph2D from 'react-force-graph-2d'

interface GraphProps {
    nodes: { id: string; name: string; type: string }[]
    links: { source: string; target: string; type: string }[]
}

export function EntityGraph({ nodes, links }: GraphProps) {
    const graphData = useMemo(() => ({
        nodes: nodes.map(n => ({ ...n, val: n.type === 'person' ? 5 : 2 })),
        links: links.map(l => ({ ...l }))
    }), [nodes, links])

    return (
        <div className="w-full h-[600px] border border-zinc-800 rounded-2xl overflow-hidden bg-zinc-950">
            <ForceGraph2D
                graphData={graphData}
                nodeLabel="name"
                nodeAutoColorBy="type"
                linkColor={() => '#3f3f46'}
                linkDirectionalArrowLength={3.5}
                linkDirectionalArrowRelPos={1}
                enableNodeDrag={true}
                backgroundColor="#09090b"
            />
        </div>
    )
}
