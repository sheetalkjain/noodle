import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Mail, Search, Settings, Share2, LayoutDashboard } from 'lucide-react'
import { SentimentChart } from './components/SentimentChart'
import { EntityGraph } from './components/EntityGraph'
import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs))
}

function App() {
    const [emails, setEmails] = useState<any[]>([])
    const [searchQuery, setSearchQuery] = useState('')
    const [activeTab, setActiveTab] = useState('dashboard')
    const [stats, setStats] = useState<any>({ total_emails: 0, sentiments: [] })
    const [graphData, setGraphData] = useState<any>({ nodes: [], links: [] })
    const [isLoading, setIsLoading] = useState(false)

    const mockSentimentData = [
        { sentiment: 'very_positive', count: 12 },
        { sentiment: 'positive', count: 25 },
        { sentiment: 'neutral', count: 40 },
        { sentiment: 'negative', count: 8 },
        { sentiment: 'very_negative', count: 3 },
    ]

    const mockGraphData = {
        nodes: [
            { id: '1', name: 'John Doe', type: 'person' },
            { id: '2', name: 'Project Noodle', type: 'project' },
            { id: '3', name: 'Microsoft', type: 'org' },
        ],
        links: [
            { source: '1', target: '2', type: 'member' },
            { source: '1', target: '3', type: 'employee' },
        ]
    }

    const fetchStats = async () => {
        try {
            const data = await invoke('get_stats')
            setStats(data)

            const graph = await invoke('get_graph')
            setGraphData(graph)
        } catch (error) {
            console.error('Failed to fetch data:', error)
        }
    }

    const startSync = async () => {
        setIsLoading(true)
        try {
            await invoke('start_sync')
            // Refresh stats periodically
            const interval = setInterval(fetchStats, 5000)
            return () => clearInterval(interval)
        } catch (error) {
            console.error('Sync failed:', error)
        } finally {
            setIsLoading(false)
        }
    }

    useEffect(() => {
        fetchStats()
    }, [])

    const handleSearch = async () => {
        try {
            const results = await invoke('search_emails', { query: searchQuery })
            setEmails(results as any[])
        } catch (error) {
            console.error('Search failed:', error)
        }
    }

    return (
        <div className="flex h-screen bg-black text-white">
            {/* Sidebar */}
            <div className="w-16 border-r border-zinc-800 flex flex-col items-center py-6 gap-8">
                <LayoutDashboard
                    className={cn("w-6 h-6 cursor-pointer", activeTab === 'dashboard' ? "text-blue-500" : "text-zinc-500")}
                    onClick={() => setActiveTab('dashboard')}
                />
                <Mail
                    className={cn("w-6 h-6 cursor-pointer", activeTab === 'emails' ? "text-blue-500" : "text-zinc-500")}
                    onClick={() => setActiveTab('emails')}
                />
                <Search
                    className={cn("w-6 h-6 cursor-pointer", activeTab === 'search' ? "text-blue-500" : "text-zinc-500")}
                    onClick={() => setActiveTab('search')}
                />
                <Share2
                    className={cn("w-6 h-6 cursor-pointer", activeTab === 'graph' ? "text-blue-500" : "text-zinc-500")}
                    onClick={() => setActiveTab('graph')}
                />
                <div className="mt-auto">
                    <Settings className="w-6 h-6 text-zinc-500 cursor-pointer" />
                </div>
            </div>

            {/* Main Content */}
            <div className="flex-1 flex flex-col">
                <header className="h-16 border-b border-zinc-800 flex items-center px-6">
                    <h1 className="text-xl font-bold tracking-tight">Noodle</h1>
                    <button
                        onClick={startSync}
                        disabled={isLoading}
                        className="ml-6 px-4 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-800 rounded-md text-sm font-medium transition-colors"
                    >
                        {isLoading ? 'Syncing...' : 'Sync Outlook'}
                    </button>
                    <div className="ml-auto w-96 relative">
                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                        <input
                            type="text"
                            placeholder="Search emails..."
                            className="w-full bg-zinc-900 border border-zinc-700 rounded-lg py-2 pl-10 pr-4 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                        />
                    </div>
                </header>

                <main className="flex-1 overflow-auto p-6">
                    {activeTab === 'dashboard' && (
                        <div className="space-y-6">
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <SentimentChart data={stats.sentiments.length > 0 ? stats.sentiments : mockSentimentData} />
                                <div className="bg-zinc-950 border border-zinc-800 rounded-2xl p-6 flex flex-col justify-center">
                                    <h4 className="text-zinc-400 text-sm mb-1">Total Emails</h4>
                                    <div className="text-4xl font-bold">{stats.total_emails.toLocaleString()}</div>
                                    <div className="text-emerald-500 text-sm mt-2">Live sync active</div>
                                </div>
                            </div>
                            <div className="rounded-2xl border border-zinc-800 p-4 bg-zinc-950">
                                <h3 className="text-sm font-medium text-zinc-400 mb-4 px-2">Relationship Graph</h3>
                                <EntityGraph nodes={graphData.nodes.length > 0 ? graphData.nodes : mockGraphData.nodes} links={graphData.links.length > 0 ? graphData.links : mockGraphData.links} />
                            </div>
                        </div>
                    )}

                    {activeTab === 'emails' && (
                        <div className="grid grid-cols-1 gap-4">
                            {emails.length === 0 ? (
                                <div className="text-center py-20 text-zinc-500">
                                    No emails found. Start indexing to see results.
                                </div>
                            ) : (
                                emails.map((email) => (
                                    <div key={email.id} className="p-4 rounded-xl border border-zinc-800 bg-zinc-950 hover:bg-zinc-900 transition-colors cursor-pointer group">
                                        <div className="flex justify-between items-start mb-2">
                                            <h3 className="font-semibold group-hover:text-blue-400 transition-colors">{email.subject}</h3>
                                            <span className="text-xs text-zinc-500">{new Date(email.received_at).toLocaleString()}</span>
                                        </div>
                                        <p className="text-sm text-zinc-400 line-clamp-2">{email.body_text}</p>
                                    </div>
                                ))
                            )}
                        </div>
                    )}
                </main>
            </div>
        </div>
    )
}

export default App
