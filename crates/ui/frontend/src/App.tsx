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
    const [logs, setLogs] = useState<string[]>([])
    const [showLogs, setShowLogs] = useState(false)

    const addLog = (message: string, type: 'info' | 'error' = 'info') => {
        const timestamp = new Date().toLocaleTimeString()
        const entry = `[${timestamp}] ${type.toUpperCase()}: ${message}`
        console.log(entry)
        setLogs(prev => [entry, ...prev].slice(0, 100))
    }

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
        addLog('Fetching stats...')
        try {
            const data = await invoke('get_stats')
            setStats(data)
            addLog('Stats updated successfully')

            const graph = await invoke('get_graph')
            setGraphData(graph)
            addLog('Graph data updated')
        } catch (error: any) {
            addLog(`Failed to fetch data: ${error}`, 'error')
        }
    }

    const startSync = async () => {
        addLog('Requesting Outlook sync...')
        setIsLoading(true)
        try {
            await invoke('start_sync')
            addLog('Sync started in background')
            // Refresh stats periodically
            const interval = setInterval(fetchStats, 5000)
            return () => clearInterval(interval)
        } catch (error: any) {
            addLog(`Sync failed: ${error}`, 'error')
        } finally {
            setIsLoading(false)
        }
    }

    useEffect(() => {
        fetchStats()

        window.onerror = (msg, _url, _lineNo, _columnNo, error) => {
            addLog(`Global Error: ${msg} ${error}`, 'error')
            return false
        }

        window.onunhandledrejection = (event) => {
            addLog(`Unhandled Promise Rejection: ${event.reason}`, 'error')
        }
    }, [])

    const handleSearch = async () => {
        addLog(`Searching for: ${searchQuery}`)
        try {
            const results = await invoke('search_emails', { query: searchQuery })
            setEmails(results as any[])
            addLog(`Search returned ${(results as any[]).length} results`)
        } catch (error: any) {
            addLog(`Search failed: ${error}`, 'error')
        }
    }

    const handleTabChange = (tab: string) => {
        addLog(`Switching tab to: ${tab}`)
        setActiveTab(tab)
    }

    return (
        <div className="flex h-screen bg-zinc-950 text-white font-sans selection:bg-blue-500/30 overflow-hidden">
            {/* Sidebar */}
            <div className="w-16 border-r border-zinc-800 flex flex-col items-center py-6 gap-6 bg-zinc-950/50 backdrop-blur-xl z-20">
                <div className="mb-2">
                    <div className="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center font-bold text-white shadow-lg shadow-blue-500/20">
                        N
                    </div>
                </div>

                <div className="w-10 h-[1px] bg-zinc-800 my-2" />

                <button
                    onClick={() => handleTabChange('dashboard')}
                    className={cn(
                        "p-3 rounded-xl transition-all duration-200 group relative",
                        activeTab === 'dashboard' ? "bg-blue-500/10 text-blue-400" : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
                    )}
                >
                    <LayoutDashboard className="w-5 h-5" />
                    {activeTab === 'dashboard' && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-500 rounded-r-full -ml-[18px]" />}
                </button>
                <button
                    onClick={() => handleTabChange('emails')}
                    className={cn(
                        "p-3 rounded-xl transition-all duration-200 group relative",
                        activeTab === 'emails' ? "bg-blue-500/10 text-blue-400" : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
                    )}
                >
                    <Mail className="w-5 h-5" />
                    {activeTab === 'emails' && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-500 rounded-r-full -ml-[18px]" />}
                </button>
                <button
                    onClick={() => handleTabChange('search')}
                    className={cn(
                        "p-3 rounded-xl transition-all duration-200 group relative",
                        activeTab === 'search' ? "bg-blue-500/10 text-blue-400" : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
                    )}
                >
                    <Search className="w-5 h-5" />
                    {activeTab === 'search' && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-500 rounded-r-full -ml-[18px]" />}
                </button>
                <button
                    onClick={() => handleTabChange('graph')}
                    className={cn(
                        "p-3 rounded-xl transition-all duration-200 group relative",
                        activeTab === 'graph' ? "bg-blue-500/10 text-blue-400" : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
                    )}
                >
                    <Share2 className="w-5 h-5" />
                    {activeTab === 'graph' && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-500 rounded-r-full -ml-[18px]" />}
                </button>

                <div className="mt-auto flex flex-col items-center gap-4">
                    <button
                        onClick={() => setShowLogs(!showLogs)}
                        className={cn("p-3 rounded-xl transition-colors", showLogs ? "text-blue-400 bg-blue-400/10" : "text-zinc-600 hover:bg-zinc-900")}
                    >
                        <LayoutDashboard className="w-5 h-5 rotate-180" />
                    </button>
                    <button className="p-3 rounded-xl text-zinc-600 hover:bg-zinc-900 hover:text-zinc-300 transition-colors">
                        <Settings className="w-5 h-5" />
                    </button>
                </div>
            </div>

            {/* Main Content */}
            <div className="flex-1 flex flex-col min-w-0 bg-zinc-950 relative">
                <header className="h-16 border-b border-zinc-800 flex items-center px-6 justify-between bg-zinc-950/80 backdrop-blur-md sticky top-0 z-10 transition-all">
                    <div className="flex items-center gap-4 min-w-[200px]">
                        <h1 className="text-lg font-semibold tracking-tight text-white">
                            {activeTab.charAt(0).toUpperCase() + activeTab.slice(1)}
                        </h1>
                        <div className="h-4 w-[1px] bg-zinc-800" />
                        <span className="text-xs text-zinc-500 font-mono">v0.1.0</span>
                    </div>

                    {/* Centered Search Bar for Symmetry */}
                    <div className="flex-1 max-w-xl mx-4 relative group">
                        <div className="absolute inset-0 bg-gradient-to-r from-blue-500/20 to-purple-500/20 rounded-lg blur opacity-0 group-focus-within:opacity-100 transition-opacity duration-500" />
                        <div className="relative">
                            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500 group-focus-within:text-blue-400 transition-colors" />
                            <input
                                type="text"
                                placeholder="Search emails semantically..."
                                className="w-full bg-zinc-900/50 border border-zinc-800 rounded-lg py-2 pl-10 pr-4 focus:outline-none focus:bg-zinc-900 focus:border-blue-500/50 text-sm transition-all placeholder:text-zinc-600"
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                            />
                        </div>
                    </div>

                    <div className="flex items-center justify-end min-w-[200px]">
                        <button
                            onClick={startSync}
                            disabled={isLoading}
                            className={cn(
                                "flex items-center gap-2 px-4 py-2 bg-white text-black hover:bg-zinc-200 disabled:bg-zinc-800 disabled:text-zinc-500 rounded-lg text-sm font-medium transition-all active:scale-95 shadow-lg shadow-white/5",
                                isLoading && "cursor-not-allowed opacity-70"
                            )}
                        >
                            {isLoading ? (
                                <>
                                    <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
                                    <span>Syncing...</span>
                                </>
                            ) : (
                                <>
                                    <div className={`w-2 h-2 rounded-full ${isLoading ? 'bg-zinc-500' : 'bg-green-500 animate-pulse'}`} />
                                    <span>Sync Outlook</span>
                                </>
                            )}
                        </button>
                    </div>
                </header>

                <main className="flex-1 overflow-auto p-8 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent">
                    {activeTab === 'dashboard' && (
                        <div className="space-y-6 max-w-7xl mx-auto animate-in fade-in slide-in-from-bottom-4 duration-500">
                            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                                <div className="lg:col-span-2 bg-zinc-900/40 border border-zinc-800/50 rounded-2xl p-1 overflow-hidden backdrop-blur-sm">
                                    <div className="p-4 border-b border-zinc-800/50 bg-zinc-900/20">
                                        <h3 className="font-medium text-zinc-300 flex items-center gap-2">
                                            <Share2 className="w-4 h-4 text-purple-400" />
                                            Sentiment Analysis
                                        </h3>
                                    </div>
                                    <div className="p-4">
                                        <SentimentChart data={stats.sentiments.length > 0 ? stats.sentiments : mockSentimentData} />
                                    </div>
                                </div>
                                <div className="space-y-6">
                                    <div className="bg-zinc-900/40 border border-zinc-800/50 rounded-2xl p-6 flex flex-col justify-center relative overflow-hidden group">
                                        <div className="absolute inset-0 bg-gradient-to-br from-blue-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
                                        <h4 className="text-zinc-400 text-sm font-medium mb-2 uppercase tracking-wider">Total Emails</h4>
                                        <div className="text-5xl font-bold tracking-tight text-white mb-2">{stats.total_emails.toLocaleString()}</div>
                                        <div className="flex items-center gap-2 text-emerald-400 text-sm bg-emerald-400/10 w-fit px-2 py-1 rounded-full border border-emerald-400/20">
                                            <div className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                                            Live sync active
                                        </div>
                                    </div>

                                    <div className="bg-zinc-900/40 border border-zinc-800/50 rounded-2xl p-6 flex flex-col justify-center">
                                        <h4 className="text-zinc-400 text-sm font-medium mb-4 uppercase tracking-wider">System Health</h4>
                                        <div className="space-y-3">
                                            <div className="flex justify-between text-sm">
                                                <span className="text-zinc-500">Database</span>
                                                <span className="text-green-400">Connected</span>
                                            </div>
                                            <div className="flex justify-between text-sm">
                                                <span className="text-zinc-500">Vector Search</span>
                                                <span className="text-blue-400">Ready</span>
                                            </div>
                                            <div className="flex justify-between text-sm">
                                                <span className="text-zinc-500">AI Engine</span>
                                                <span className="text-zinc-300">Local (Ollama/LM)</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div className="rounded-2xl border border-zinc-800/50 bg-zinc-900/40 backdrop-blur-sm overflow-hidden flex flex-col h-[500px]">
                                <div className="p-4 border-b border-zinc-800/50 bg-zinc-900/20 flex justify-between items-center">
                                    <h3 className="font-medium text-zinc-300 flex items-center gap-2">
                                        <Share2 className="w-4 h-4 text-blue-400" />
                                        Entity Relationship Graph
                                    </h3>
                                    <span className="text-xs text-zinc-500">Interactive Visualization</span>
                                </div>
                                <div className="flex-1 relative">
                                    <EntityGraph nodes={graphData.nodes.length > 0 ? graphData.nodes : mockGraphData.nodes} links={graphData.links.length > 0 ? graphData.links : mockGraphData.links} />
                                </div>
                            </div>
                        </div>
                    )}

                    {activeTab === 'emails' && (
                        <div className="grid grid-cols-1 gap-4 max-w-4xl mx-auto animate-in fade-in slide-in-from-bottom-8 duration-500">
                            {emails.length === 0 ? (
                                <div className="flex flex-col items-center justify-center py-32 text-zinc-500 border-2 border-dashed border-zinc-800 rounded-3xl bg-zinc-900/20">
                                    <Search className="w-12 h-12 mb-4 text-zinc-700" />
                                    <p className="text-lg font-medium text-zinc-400">No emails found</p>
                                    <p className="text-sm">Try running a sync or changing your search query.</p>
                                </div>
                            ) : (
                                emails.map((email) => (
                                    <div key={email.id} className="p-5 rounded-xl border border-zinc-800 bg-zinc-900/40 hover:bg-zinc-900/80 hover:border-zinc-700/80 transition-all cursor-pointer group shadow-sm hover:shadow-xl hover:shadow-black/50 hover:-translate-y-0.5">
                                        <div className="flex justify-between items-start mb-3">
                                            <h3 className="font-semibold text-lg text-zinc-200 group-hover:text-blue-400 transition-colors">{email.subject}</h3>
                                            <span className="text-xs font-mono text-zinc-500 bg-zinc-950 px-2 py-1 rounded border border-zinc-800">{new Date(email.received_at).toLocaleDateString()}</span>
                                        </div>
                                        <div className="flex items-center gap-2 mb-3">
                                            <div className="w-6 h-6 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-[10px] uppercase font-bold text-white shadow-inner">
                                                {email.sender.substring(0, 2)}
                                            </div>
                                            <span className="text-xs text-zinc-400">{email.sender}</span>
                                        </div>
                                        <p className="text-sm text-zinc-400 line-clamp-2 leading-relaxed opacity-80 group-hover:opacity-100 transition-opacity">{email.body_text}</p>
                                    </div>
                                ))
                            )}
                        </div>
                    )}
                </main>

                {/* Log Overlay */}
                {showLogs && (
                    <div className="absolute bottom-6 right-6 w-96 max-h-[400px] bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-2xl shadow-2xl flex flex-col z-50 animate-in fade-in zoom-in-95 duration-200 origin-bottom-right">
                        <div className="p-4 border-b border-zinc-800 flex justify-between items-center bg-zinc-900/50">
                            <h3 className="text-xs font-bold uppercase tracking-widest text-zinc-500">System Logs</h3>
                            <button onClick={() => setLogs([])} className="text-[10px] text-zinc-600 hover:text-zinc-400 uppercase font-bold">Clear</button>
                        </div>
                        <div className="flex-1 overflow-auto p-4 space-y-2 font-mono text-[11px] selection:bg-blue-500/20">
                            {logs.length === 0 ? (
                                <div className="text-zinc-700 italic">No logs yet...</div>
                            ) : (
                                logs.map((log, i) => (
                                    <div key={i} className={cn(
                                        "p-2 rounded border border-zinc-800 bg-zinc-950/50",
                                        log.includes('ERROR') ? "text-red-400 border-red-500/20" : "text-zinc-400"
                                    )}>
                                        {log}
                                    </div>
                                ))
                            )}
                        </div>
                    </div>
                )}
            </div>
        </div>
    ) // End App
}

export default App
