import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Mail, Search, Settings, Share2, LayoutDashboard } from 'lucide-react'
import { SentimentChart } from './components/SentimentChart'
import { EntityGraph } from './components/EntityGraph'
import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs))
}

function App() {
    const [hasLoadedInitialEmails, setHasLoadedInitialEmails] = useState(false)
    const [emails, setEmails] = useState<any[]>([])
    const [searchQuery, setSearchQuery] = useState('')
    const [activeTab, setActiveTab] = useState('dashboard')
    const [stats, setStats] = useState<any>({ total_emails: 0, sentiments: [] })
    const [graphData, setGraphData] = useState<any>({ nodes: [], links: [] })
    const [isLoading, setIsLoading] = useState(false)
    const [logs, setLogs] = useState<any[]>([])
    const [config, setConfig] = useState<any>({
        ollama_url: 'http://localhost:11434',
        model_name: 'llama3',
        sync_interval: '2',
        history_days: '90',
        provider_type: 'ollama',
        api_key: '',
        confirm_exit: 'true'
    })
    const [availableModels, setAvailableModels] = useState<string[]>([])
    const [showExitConfirm, setShowExitConfirm] = useState(false)

    const addLog = async (message: string, type: 'info' | 'error' | 'warn' = 'info') => {
        const timestamp = new Date().toISOString()
        const entry = { timestamp, level: type.toUpperCase(), source: 'FRONTEND', message }
        console.log(`[${type.toUpperCase()}] ${message}`)
        setLogs(prev => [entry, ...prev].slice(0, 1000))

        // Optionally persist frontend logs too
        try {
            await invoke('save_log_cmd', { level: type.toUpperCase(), source: 'FRONTEND', message })
        } catch (e) {
            // ignore
        }
    }


    const fetchStats = async () => {
        try {
            const data = await invoke('get_stats')
            setStats(data)

            const graph = await invoke('get_graph')
            setGraphData(graph)

            const recentLogs = await invoke('get_logs', { limit: 100 })
            setLogs(recentLogs as any[])
        } catch (error: any) {
            console.error(`Failed to fetch data: ${error}`)
        }
    }

    const fetchConfig = async () => {
        try {
            const ollama = await invoke('get_config', { key: 'ollama_url' })
            const model = await invoke('get_config', { key: 'model_name' })
            const interval = await invoke('get_config', { key: 'sync_interval' })
            const history = await invoke('get_config', { key: 'history_days' })
            const provider = await invoke('get_config', { key: 'provider_type' })
            const apiKey = await invoke('get_config', { key: 'api_key' })
            const confirm = await invoke('get_config', { key: 'confirm_exit' })

            if (ollama || model || interval || history || provider || apiKey || confirm) {
                setConfig({
                    ollama_url: ollama || config.ollama_url,
                    model_name: model || config.model_name,
                    sync_interval: interval || config.sync_interval,
                    history_days: history || config.history_days,
                    provider_type: provider || 'ollama',
                    api_key: apiKey || '',
                    confirm_exit: confirm || 'true'
                })
            }
        } catch (e) {
            addLog(`Failed to fetch config: ${e}`, 'error')
        }
    }

    const startSync = async () => {
        addLog('Requesting Outlook sync...')
        setIsLoading(true)
        try {
            await invoke('start_sync')
            addLog('Sync thread spawned successfully')
            // Refresh stats periodically for 2 minutes or until emails start appearing
            const intervalId = setInterval(fetchStats, 5000)
            setTimeout(() => clearInterval(intervalId), 120000)
        } catch (error: any) {
            addLog(`Sync failed: ${error}`, 'error')
            setIsLoading(false)
        }
    }

    useEffect(() => {
        fetchStats()
        fetchConfig()

        const unlistenPromise = listen('noodle://log', (event: any) => {
            const { message, level } = event.payload
            const entry = {
                timestamp: new Date().toISOString(),
                level: level.toUpperCase(),
                source: 'BACKEND',
                message
            }
            setLogs(prev => [entry, ...prev].slice(0, 1000))
        })

        const unlistenExit = listen('noodle://show-exit-confirm', () => {
            setShowExitConfirm(true)
        })

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.altKey && e.key === 'F4') {
                e.preventDefault()
                invoke('request_exit').catch(err => console.error(err))
            }
        }
        window.addEventListener('keydown', handleKeyDown)

        window.onerror = (msg, _url, _lineNo, _columnNo, error) => {
            addLog(`Global Error: ${msg} ${error}`, 'error')
            return false
        }

        window.onunhandledrejection = (event) => {
            addLog(`Unhandled Promise Rejection: ${event.reason}`, 'error')
        }

        return () => {
            unlistenPromise.then(unlisten => unlisten())
            unlistenExit.then(unlisten => unlisten())
            window.removeEventListener('keydown', handleKeyDown)
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
        if (tab === 'emails' && !hasLoadedInitialEmails) {
            handleSearch()
            setHasLoadedInitialEmails(true)
        }
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

                <button
                    onClick={() => handleTabChange('logs')}
                    className={cn(
                        "p-3 rounded-xl transition-all duration-200 group relative",
                        activeTab === 'logs' ? "bg-blue-500/10 text-blue-400" : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
                    )}
                >
                    <LayoutDashboard className="w-5 h-5 rotate-180" />
                    {activeTab === 'logs' && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-500 rounded-r-full -ml-[18px]" />}
                </button>

                <div className="mt-auto flex flex-col items-center gap-4">
                    <button
                        onClick={() => handleTabChange('settings')}
                        className={cn(
                            "p-3 rounded-xl transition-all duration-200 group relative",
                            activeTab === 'settings' ? "bg-blue-500/10 text-blue-400" : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
                        )}
                    >
                        <Settings className="w-5 h-5" />
                        {activeTab === 'settings' && <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-500 rounded-r-full -ml-[18px]" />}
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
                                        <SentimentChart data={stats.sentiments} />
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
                                    <EntityGraph nodes={graphData.nodes} links={graphData.links} />
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

                    {activeTab === 'logs' && (
                        <div className="max-w-7xl mx-auto animate-in fade-in slide-in-from-bottom-4 duration-500 h-full flex flex-col">
                            <div className="flex justify-between items-center mb-6">
                                <h2 className="text-2xl font-bold">System Logs</h2>
                                <button onClick={() => setLogs([])} className="px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm hover:bg-zinc-800 transition-colors">Clear All</button>
                            </div>
                            <div className="flex-1 overflow-auto border border-zinc-800 rounded-2xl bg-zinc-900/20 backdrop-blur-sm">
                                <table className="w-full text-left border-collapse">
                                    <thead className="sticky top-0 bg-zinc-900 border-b border-zinc-800 z-10 text-xs font-bold uppercase tracking-widest text-zinc-500">
                                        <tr>
                                            <th className="p-4 w-48">Timestamp</th>
                                            <th className="p-4 w-24">Level</th>
                                            <th className="p-4 w-32">Source</th>
                                            <th className="p-4">Message</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-zinc-800/50 font-mono text-[13px]">
                                        {logs.map((log, i) => (
                                            <tr key={i} className="hover:bg-zinc-900/40 transition-colors group">
                                                <td className="p-4 text-zinc-500">
                                                    {new Date(log.timestamp).toLocaleTimeString()}
                                                </td>
                                                <td className="p-4">
                                                    <span className={cn(
                                                        "px-2 py-0.5 rounded-full text-[10px] font-bold border",
                                                        log.level === 'ERROR' ? "bg-red-500/10 text-red-500 border-red-500/20" :
                                                            log.level === 'WARN' ? "bg-amber-500/10 text-amber-500 border-amber-500/20" :
                                                                "bg-blue-500/10 text-blue-500 border-blue-500/20"
                                                    )}>
                                                        {log.level}
                                                    </span>
                                                </td>
                                                <td className="p-4 text-zinc-400 uppercase text-[11px] font-bold tracking-tight">{log.source}</td>
                                                <td className="p-4 text-zinc-300 group-hover:text-white transition-colors">{log.message}</td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    )}

                    {activeTab === 'settings' && (
                        <div className="max-w-3xl mx-auto animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-8">
                            <div>
                                <h2 className="text-3xl font-bold mb-2">Settings</h2>
                                <p className="text-zinc-500">Configure your AI agent and data synchronization preferences.</p>
                            </div>

                            <div className="space-y-6">
                                <section className="bg-zinc-900/40 border border-zinc-800/50 rounded-2xl p-6 space-y-6">
                                    <h3 className="text-lg font-medium flex items-center gap-2">
                                        <Settings className="w-5 h-5 text-blue-400" />
                                        AI Provider
                                    </h3>
                                    <div className="grid grid-cols-1 gap-4">
                                        <div className="space-y-2">
                                            <label className="text-sm text-zinc-400">Provider Type</label>
                                            <select
                                                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 focus:border-blue-500 outline-none transition-all"
                                                value={config.provider_type}
                                                onChange={(e) => setConfig({ ...config, provider_type: e.target.value })}
                                            >
                                                <option value="ollama">Ollama (Local)</option>
                                                <option value="openai">Lemonade / Foundry / OpenAI (Compatible)</option>
                                            </select>
                                        </div>

                                        <div className="space-y-2">
                                            <label className="text-sm text-zinc-400">{config.provider_type === 'ollama' ? 'Ollama URL' : 'Base URL'}</label>
                                            <input
                                                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 focus:border-blue-500 outline-none transition-all"
                                                value={config.ollama_url}
                                                onChange={(e) => setConfig({ ...config, ollama_url: e.target.value })}
                                                placeholder={config.provider_type === 'ollama' ? "http://localhost:11434" : "https://api.openai.com/v1"}
                                            />
                                        </div>

                                        {config.provider_type === 'openai' && (
                                            <div className="space-y-2">
                                                <label className="text-sm text-zinc-400">API Key</label>
                                                <input
                                                    type="password"
                                                    className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 focus:border-blue-500 outline-none transition-all"
                                                    value={config.api_key}
                                                    onChange={(e) => setConfig({ ...config, api_key: e.target.value })}
                                                    placeholder="sk-..."
                                                />
                                            </div>
                                        )}

                                        <div className="space-y-2">
                                            <label className="text-sm text-zinc-400">Model Name</label>
                                            <div className="flex gap-2">
                                                <input
                                                    list="model-suggestions"
                                                    className="flex-1 bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 focus:border-blue-500 outline-none transition-all"
                                                    value={config.model_name}
                                                    onChange={(e) => setConfig({ ...config, model_name: e.target.value })}
                                                    placeholder="e.g. gpt-4 or llama3"
                                                />
                                                <datalist id="model-suggestions">
                                                    {availableModels.map(m => <option key={m} value={m} />)}
                                                </datalist>
                                                <button
                                                    onClick={async () => {
                                                        addLog(`Fetching models from ${config.ollama_url}...`)
                                                        try {
                                                            // Temporarily save config to ensure backend uses correct credentials for fetch
                                                            await invoke('save_config', { key: 'ollama_url', value: config.ollama_url })
                                                            await invoke('save_config', { key: 'provider_type', value: config.provider_type })
                                                            await invoke('save_config', { key: 'api_key', value: config.api_key })

                                                            const models = await invoke('get_models') as string[]
                                                            setAvailableModels(models)
                                                            addLog(`Found ${models.length} models`)
                                                        } catch (e: any) {
                                                            addLog(`Failed to fetch models: ${e}`, 'error')
                                                        }
                                                    }}
                                                    className="bg-zinc-800 hover:bg-zinc-700 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors"
                                                >
                                                    Fetch Models
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                </section>

                                <section className="bg-zinc-900/40 border border-zinc-800/50 rounded-2xl p-6 space-y-6">
                                    <h3 className="text-lg font-medium flex items-center gap-2">
                                        <Mail className="w-5 h-5 text-purple-400" />
                                        Outlook Sync
                                    </h3>
                                    <div className="grid grid-cols-2 gap-4">
                                        <div className="space-y-2">
                                            <label className="text-sm text-zinc-400">Interval (minutes)</label>
                                            <input
                                                type="number"
                                                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 focus:border-blue-500 outline-none transition-all"
                                                value={config.sync_interval}
                                                onChange={(e) => setConfig({ ...config, sync_interval: e.target.value })}
                                            />
                                        </div>
                                        <div className="space-y-2">
                                            <label className="text-sm text-zinc-400">History to sync (days)</label>
                                            <input
                                                type="number"
                                                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 focus:border-blue-500 outline-none transition-all"
                                                value={config.history_days}
                                                onChange={(e) => setConfig({ ...config, history_days: e.target.value })}
                                            />
                                        </div>
                                    </div>

                                    <div className="pt-4 border-t border-zinc-800/50">
                                        <label className="flex items-center gap-3 cursor-pointer group">
                                            <div className="relative">
                                                <input
                                                    type="checkbox"
                                                    className="peer sr-only"
                                                    checked={config.confirm_exit !== 'false'}
                                                    onChange={(e) => setConfig({ ...config, confirm_exit: e.target.checked ? 'true' : 'false' })}
                                                />
                                                <div className="w-10 h-6 bg-zinc-800 rounded-full peer-checked:bg-blue-600 transition-colors" />
                                                <div className="absolute left-1 top-1 w-4 h-4 bg-white rounded-full transition-transform peer-checked:translate-x-4" />
                                            </div>
                                            <span className="text-sm text-zinc-300 group-hover:text-white transition-colors">Confirm before exiting app</span>
                                        </label>
                                    </div>
                                </section>

                                <div className="flex justify-end gap-3">
                                    <button
                                        onClick={async () => {
                                            try {
                                                await invoke('save_config', { key: 'ollama_url', value: config.ollama_url })
                                                await invoke('save_config', { key: 'model_name', value: config.model_name })
                                                await invoke('save_config', { key: 'sync_interval', value: config.sync_interval })
                                                await invoke('save_config', { key: 'history_days', value: config.history_days })
                                                await invoke('save_config', { key: 'provider_type', value: config.provider_type })
                                                await invoke('save_config', { key: 'api_key', value: config.api_key })
                                                await invoke('save_config', { key: 'confirm_exit', value: config.confirm_exit })
                                                addLog('Settings saved successfully')
                                            } catch (e) {
                                                addLog(`Failed to save settings: ${e}`, 'error')
                                            }
                                        }}
                                        className="bg-blue-600 hover:bg-blue-500 text-white px-8 py-2 rounded-lg font-medium transition-all active:scale-95"
                                    >
                                        Save Changes
                                    </button>
                                </div>
                            </div>
                        </div>
                    )}
                </main>
            </div >

            {showExitConfirm && (
                <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-in fade-in duration-200">
                    <div className="bg-zinc-900 border border-zinc-800 rounded-2xl p-6 max-w-sm w-full shadow-2xl scale-100 animate-in zoom-in-95 duration-200">
                        <h3 className="text-lg font-bold text-white mb-2">Quit Application?</h3>
                        <p className="text-zinc-400 text-sm mb-6">
                            This will stop the AI agent and Outlook sync. Are you sure you want to quit?
                        </p>
                        <div className="flex justify-end gap-3">
                            <button
                                onClick={() => setShowExitConfirm(false)}
                                className="px-4 py-2 rounded-lg text-sm font-medium text-zinc-300 hover:text-white hover:bg-zinc-800 transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={() => invoke('force_exit')}
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-red-500/10 text-red-500 hover:bg-red-500/20 border border-red-500/20 transition-colors"
                            >
                                Quit Noodle
                            </button>
                        </div>
                    </div>
                </div>
            )
            }
        </div >
    ) // End App
}

export default App
