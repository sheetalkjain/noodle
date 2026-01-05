import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts'

interface SentimentData {
    sentiment: string
    count: number
}

interface SentimentChartProps {
    data: SentimentData[]
    onSentimentClick?: (sentiment: string) => void
    selectedSentiment?: string | null
}

const COLORS: Record<string, string> = {
    very_positive: '#10b981',
    positive: '#34d399',
    neutral: '#94a3b8',
    negative: '#f87171',
    very_negative: '#ef4444',
}

const SELECTED_COLORS: Record<string, string> = {
    very_positive: '#059669',
    positive: '#10b981',
    neutral: '#64748b',
    negative: '#dc2626',
    very_negative: '#b91c1c',
}

export function SentimentChart({ data, onSentimentClick, selectedSentiment }: SentimentChartProps) {
    const handleBarClick = (entry: SentimentData) => {
        if (onSentimentClick) {
            // If clicking the same sentiment, clear the filter
            if (selectedSentiment === entry.sentiment) {
                onSentimentClick('')
            } else {
                onSentimentClick(entry.sentiment)
            }
        }
    }

    return (
        <div className="w-full h-[300px] p-4 bg-zinc-950 border border-zinc-800 rounded-2xl">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-sm font-medium text-zinc-400">Sentiment Distribution</h3>
                {selectedSentiment && (
                    <button
                        onClick={() => onSentimentClick?.('')}
                        className="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1"
                    >
                        <span>Clear filter</span>
                        <span className="text-zinc-500">×</span>
                    </button>
                )}
            </div>
            <ResponsiveContainer width="100%" height="100%">
                <BarChart data={data}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#27272a" vertical={false} />
                    <XAxis
                        dataKey="sentiment"
                        stroke="#71717a"
                        fontSize={12}
                        tickLine={false}
                        axisLine={false}
                    />
                    <YAxis
                        stroke="#71717a"
                        fontSize={12}
                        tickLine={false}
                        axisLine={false}
                    />
                    <Tooltip
                        contentStyle={{ backgroundColor: '#18181b', border: '1px solid #27272a', borderRadius: '8px' }}
                        itemStyle={{ color: '#fff' }}
                        content={({ payload }) => {
                            if (!payload || payload.length === 0) return null
                            const entry = payload[0].payload as SentimentData
                            return (
                                <div className="bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-2 shadow-lg">
                                    <p className="text-white font-medium">{entry.sentiment}</p>
                                    <p className="text-zinc-400 text-sm">{entry.count} emails</p>
                                    <p className="text-blue-400 text-xs mt-1">Click to filter</p>
                                </div>
                            )
                        }}
                    />
                    <Bar
                        dataKey="count"
                        radius={[4, 4, 0, 0]}
                        cursor="pointer"
                    >
                        {data.map((entry, index) => (
                            <Cell
                                key={`cell-${index}`}
                                fill={
                                    selectedSentiment === entry.sentiment
                                        ? SELECTED_COLORS[entry.sentiment] || '#2563eb'
                                        : COLORS[entry.sentiment] || '#3b82f6'
                                }
                                stroke={selectedSentiment === entry.sentiment ? '#fff' : 'none'}
                                strokeWidth={selectedSentiment === entry.sentiment ? 2 : 0}
                                onClick={() => handleBarClick(entry)}
                                style={{
                                    opacity: selectedSentiment && selectedSentiment !== entry.sentiment ? 0.4 : 1,
                                    transition: 'opacity 0.2s, fill 0.2s'
                                }}
                            />
                        ))}
                    </Bar>
                </BarChart>
            </ResponsiveContainer>
        </div>
    )
}
