import { useCallback, useEffect, useState } from 'react';
import { ArrowUpRight, CalendarDays, ChevronRight, Loader2, RefreshCw, X } from 'lucide-react';
import { open } from '@tauri-apps/plugin-shell';
import { calendarDate, calendarEvents, calendarStatus, calendarTime, cancelCalendar, connectCalendar, disconnectCalendar, type CalendarAgenda, type CalendarEvent, type CalendarStatus } from '../lib/calendar';
import type { MeetingSummary } from '../lib/types';
import { formatRelativeTime } from '../lib/utils';

export function TodayView({ meetings, onSelectMeeting, onStartEvent, onHistory }: {
  meetings: MeetingSummary[]; onSelectMeeting: (id: string) => void;
  onStartEvent: (event: CalendarEvent) => void; onHistory: () => void;
}) {
  const [status, setStatus] = useState<CalendarStatus | null>(null);
  const [agenda, setAgenda] = useState<CalendarAgenda | null>(null);
  const [loading, setLoading] = useState(true);
  const [connecting, setConnecting] = useState(false);
  const [setup, setSetup] = useState(false);
  const [error, setError] = useState('');
  const [clientId, setClientId] = useState('');
  const [clientSecret, setClientSecret] = useState('');
  const refresh = useCallback(async () => {
    setLoading(true); setError('');
    try {
      const next = await calendarStatus(); setStatus(next);
      if (next.clientId) setClientId(next.clientId);
      if (next.connected) setAgenda(await calendarEvents()); else setAgenda(null);
    } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  const connect = async () => {
    setConnecting(true); setError('');
    try { await connectCalendar(clientId.trim(), clientSecret.trim()); setClientSecret(''); setSetup(false); await refresh(); }
    catch (e) { setError(String(e)); } finally { setConnecting(false); }
  };
  const disconnect = async () => {
    setLoading(true); setError('');
    try { await disconnectCalendar(); setAgenda(null); setStatus({ connected: false, scope: 'Read-only calendar events' }); setSetup(false); }
    catch (e) { setError(String(e)); } finally { setLoading(false); }
  };
  const openLink = async (url: string) => { try { await open(url); } catch { setError('Could not open your browser. Try again.'); } };
  const groups = new Map<string, CalendarEvent[]>();
  for (const event of agenda?.events ?? []) {
    const date = calendarDate(event);
    const label = date.toDateString() === new Date().toDateString() ? 'Today' : date.toLocaleDateString([], { weekday: 'long', month: 'short', day: 'numeric' });
    groups.set(label, [...(groups.get(label) ?? []), event]);
  }
  return <div className="today-view">
    <header className="today-heading"><div><p>{new Date().toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' })}</p><h1>Your meetings</h1></div><CalendarDays aria-hidden="true" size={24} /></header>
    <section aria-label="Upcoming meetings" className="agenda-section">
      <div className="section-heading"><h2>Coming up</h2><div className="inline-actions">
        {status?.connected && <button className="quiet-button" disabled={loading || connecting} onClick={refresh} aria-label="Refresh calendar"><RefreshCw size={14} className={loading ? 'animate-spin' : ''} /></button>}
        <button className="quiet-button" onClick={() => setSetup(v => !v)} aria-expanded={setup}>{status?.connected ? 'Calendar settings' : 'Connect calendar'}</button>
      </div></div>
      {error && <div role="alert" className="calendar-error"><p>{error}</p><button className="quiet-button" onClick={refresh} disabled={loading || connecting}>Retry</button></div>}
      {setup && <form className="calendar-setup" onSubmit={e => { e.preventDefault(); void connect(); }}>
        <div className="section-heading"><h3>Google Calendar</h3><button type="button" className="quiet-button" aria-label="Close calendar setup" onClick={() => setSetup(false)}><X size={16} /></button></div>
        <p>See your next seven days and start a meeting with the right project. Read-only access to your primary calendar.</p>
        <details><summary>One-time Google Cloud setup</summary><ol>
          <li>In Google Cloud, enable the Google Calendar API.</li>
          <li>Configure the OAuth consent screen. If the app is in testing, add your Google account as a test user.</li>
          <li>Create an OAuth client with application type <strong>Desktop app</strong>. Copy its client ID and client secret below.</li>
        </ol><button type="button" className="text-link" onClick={() => openLink('https://console.cloud.google.com/apis/credentials')}>Open Google Cloud <ArrowUpRight size={13} /></button><p>AI Studio API keys cannot authorize Calendar. Google test-mode authorization may expire after seven days.</p></details>
        <label>OAuth client ID<input required autoComplete="off" value={clientId} onChange={e => setClientId(e.target.value)} placeholder="…apps.googleusercontent.com" disabled={connecting} /></label>
        <label>OAuth client secret<input type="password" autoComplete="off" value={clientSecret} onChange={e => setClientSecret(e.target.value)} placeholder="From your Desktop app client" disabled={connecting} /></label>
        <p className="setup-note">Sign-in opens in your browser. On macOS, credentials stay in your private local file without Keychain prompts.</p>
        <div className="inline-actions"><button type="submit" className="solid-button" disabled={connecting || loading || !clientId.trim()}>{connecting ? <><Loader2 size={14} className="animate-spin" /> Waiting for Google…</> : 'Connect with Google'}</button>
          {connecting && <button type="button" className="quiet-button" onClick={() => cancelCalendar().catch(e => setError(String(e)))}>Cancel</button>}
          {status?.connected && <button type="button" className="quiet-button" disabled={loading || connecting} onClick={disconnect}>Disconnect from this device</button>}
        </div>
      </form>}
      {loading && !agenda ? <div className="agenda-loading" role="status">Loading your calendar…</div> : !status?.connected ? !setup && <div className="calendar-empty"><CalendarDays size={22} aria-hidden="true" /><div><h3>A little preparation, before every call.</h3><p>Connect Google Calendar to bring your upcoming meetings here. You can also start a meeting anytime.</p><button className="text-link" onClick={() => setSetup(true)}>Set up Google Calendar <ChevronRight size={14} /></button></div></div> : <>
        {agenda && error && <p className="setup-note">Showing the last successful refresh. These events may have changed.</p>}
        {agenda?.events.length === 0 && <p className="empty-copy">No upcoming events in your primary calendar for the next seven days.</p>}
        {Array.from(groups, ([label, events]) => <div key={label} className="agenda-day"><h3>{label}</h3>{events.map(event => <article className="agenda-row" key={event.id}>
          <div className="agenda-time">{calendarTime(event)}</div><div className="agenda-title"><h4>{event.title}</h4><p>{event.attendeeCount > 0 ? `${event.attendeeCount} participants` : 'Personal event'}</p></div>
          <div className="agenda-actions">{event.joinUrl && <button className="quiet-button" onClick={() => openLink(event.joinUrl!)} aria-label={`Join ${event.title}`}>Join <ArrowUpRight size={13} /></button>}<button className="outline-button" onClick={() => onStartEvent(event)} aria-label={`Prepare ${event.title}`}>Prepare</button></div>
        </article>)}</div>)}
        {agenda && <p className="setup-note">Primary calendar · Times shown in {Intl.DateTimeFormat().resolvedOptions().timeZone} · Updated {new Date(agenda.syncedAt).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}{agenda.truncated ? ' · More events exist; open Google Calendar for the full list.' : ''}</p>}
      </>}
    </section>
    <section aria-label="Recent meetings" className="recent-section"><div className="section-heading"><h2>Recent notes</h2><button className="quiet-button" onClick={onHistory}>View all <ChevronRight size={14} /></button></div>
      {meetings.length === 0 ? <p className="empty-copy">Your meeting notes, answers and decisions will appear here after your first conversation.</p> : meetings.slice(0, 5).map(meeting => <button key={meeting.id} className="recent-note" onClick={() => onSelectMeeting(meeting.id)}><span>{meeting.title || 'Untitled meeting'}</span><span>{formatRelativeTime(meeting.start_time)}<ChevronRight size={14} /></span></button>)}
    </section>
  </div>;
}
