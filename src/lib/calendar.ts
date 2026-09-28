import { invoke } from '@tauri-apps/api/core';
export interface CalendarStatus { connected: boolean; clientId?: string; scope: string }
export interface CalendarEvent { id: string; title: string; start: string; end: string; allDay: boolean; joinUrl?: string; htmlUrl?: string; attendeeCount: number }
export interface CalendarAgenda { events: CalendarEvent[]; syncedAt: string; truncated: boolean }
export const calendarStatus = () => invoke<CalendarStatus>('calendar_status');
export const connectCalendar = (clientId: string, clientSecret: string) => invoke<void>('calendar_connect', { clientId, clientSecret });
export const cancelCalendar = () => invoke<void>('calendar_cancel');
export const disconnectCalendar = () => invoke<void>('calendar_disconnect');
export const calendarEvents = () => invoke<CalendarAgenda>('calendar_events');
export function calendarDate(event: CalendarEvent): Date {
  return new Date(event.allDay ? `${event.start}T12:00:00` : event.start);
}
export function calendarTime(event: CalendarEvent): string {
  if (event.allDay) return 'All day';
  const format = (date: string) => new Date(date).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  return `${format(event.start)} – ${format(event.end)}`;
}
