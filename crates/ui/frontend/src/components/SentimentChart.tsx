import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts'

interface SentimentData {
    sentiment: string
    count: number
}

const COLORS: Record<string, string> = {
    very_positive: '#10b981',
    positive: '#34d399',
    neutral: '#94a3b8',
    negative: '#f87171',
    very_negative: '#ef4444',
}

export function SentimentChart({ data }: { data: SentimentData[] }) {
    return (
        <div className="w-full h-[300px] p-4 bg-zinc-950 border border-zinc-800 rounded-2xl">
            <h3 className="text-sm font-medium text-zinc-400 mb-4">Sentiment Distribution</h3>
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
                    />
                    <Bar dataKey="count" radius={[4, 4, 0, 0]}>
                        {data.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={COLORS[entry.sentiment] || '#3b82f6'} />
                        ))}
                    </Bar>
                </BarChart>
            </ResponsiveContainer>
        </div>
    )
}
