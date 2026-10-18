# copilot-powerline

A status line for GitHub Copilot CLI that keeps context usage, spend, prompt-cache health and session activity visible while you work.

## Sessions and activity

**Session**:
One Copilot CLI conversation, identified by its session id.
_Avoid_: Chat, conversation

**Turn**:
One user prompt and everything Copilot does in response to it. A Turn contains one or more Steps.
_Avoid_: Request, exchange

**Step**:
A single model call within a Session, sub-agent calls included; the smallest unit of activity the status line can tell apart.
_Avoid_: Call, turn, request

**Refresh**:
One run of the status line, triggered by Copilot CLI on an event or on its refresh timer. A Refresh shows a new Step only if the Session's counters moved since the previous one.
_Avoid_: Tick, update, render

**Idle Time**:
How long the Session has gone without a Step. Typing, reading and long tool runs all count as idle.
_Avoid_: Inactivity, break time

## Context and tokens

**Context Window**:
The tokens the model currently holds for the Session, measured against the model's context limit.
_Avoid_: Tokens (as the name of this concept), prompt size

**Total Session Tokens**:
All tokens exchanged in the Session (input, output and cached) across every Turn and compaction. Unlike the Context Window, it only grows.
_Avoid_: Total tokens, throughput

**Reasoning Tokens**:
Tokens a model spends thinking before it answers, accumulated over the Session.
_Avoid_: Think tokens, thinking budget

## Event data

**Event Log**:
Copilot's own record of what happened in a Session, one entry per event. It carries what the status line payload does not, such as per-Step usage and tool runs.
_Avoid_: Transcript, session file, history

**Usage Record**:
The Event Log's entry for one Step: its model, token counts and the size of the tool definitions at that moment.
_Avoid_: Step record, call record

**Tool Overhead**:
The tokens the tool definitions (built-in and MCP) add to every request. It is read from the latest Step of the main agent, because sub-agents get their own tool set.
_Avoid_: Tool tokens, MCP cost, tool size

## Prompt cache

**Prompt Cache**:
Copilot's cache of recently sent context; tokens read from it cost less than tokens sent fresh.
_Avoid_: Cache (unqualified, when other caches are in play)

**Cache Hit Rate**:
The share of the Session's prompt tokens served from the Prompt Cache.
_Avoid_: Cache ratio, cache efficiency

**Cache Trend**:
Whether the latest Step hit the Prompt Cache clearly more or clearly less than the Session average before it.

**Cache Miss**:
A Step that sent a large number of input tokens not read from the Prompt Cache, such as the first Step after the cache expired.
_Avoid_: Cold start, cache flush

**Cold Cache**:
A Prompt Cache that has most likely expired, so the next Step re-reads the whole Context Window at full price. Inferred from Idle Time, never observed directly.
_Avoid_: Expired cache, cache expiry

**Long-Break Reminder**:
The warning shown once Idle Time passes the Prompt Cache lifetimes, saying the cache is likely cold.
_Avoid_: Cache expiry, TTL warning

## Spend

**AI Credit**:
The unit Copilot bills model usage in. Its list price is USD 0.01; what a user actually pays depends on their plan.
_Avoid_: AIC (in prose), AIU, credit

**Session Spend**:
The AI Credits consumed by the current Session so far, shown in USD at list price.
_Avoid_: Session cost, session usage

**Month Spend**:
A local estimate of this CLI's AI Credit spend in the current UTC calendar month: every other Session from the 1st of the month, plus the whole current Session even if it started earlier.
_Avoid_: Monthly cost, month-to-date usage

**Cycle**:
GitHub's billing period for AI Credits: the UTC calendar month, resetting at midnight UTC on the 1st.
_Avoid_: Billing month, period

**Cycle Spend**:
GitHub's own count of the AI Credits used in the current Cycle, across every Copilot surface (IDE, github.com, CLI), not just this CLI. It covers the same period as Month Spend but is recorded by GitHub, so it can briefly trail Month Spend after a Step; otherwise the two differ by the usage on other surfaces.
_Avoid_: Cycle cost, cycle usage, GitHub usage

**Spend Alert**:
The flag raised when Session, Month or Cycle Spend goes above a limit the user set.
_Avoid_: Budget warning, limit alert

**Spend Spike**:
A Step that cost much more per token than the Session average, a hint of pricier model routing or a Cache Miss on a large context.
_Avoid_: Cost spike, expensive step

## Presentation

**Segment**:
One item of the status line, such as Context Window or Session Spend, that the user can enable, order and configure.
_Avoid_: Widget, field, module

**Display Mode**:
Which list of Segments the status line shows: Full Mode shows them all, Compact Mode a shorter list for normal work.
_Avoid_: View, layout

**Style**:
How Segments are framed and separated (minimal, powerline, capsule or plain).
_Avoid_: Layout, skin

**Theme**:
The colour palette applied to Segments.
_Avoid_: Colour scheme, skin

**Icon Set**:
How each Segment is labelled: Nerd Font glyphs, emoji, or plain text.
_Avoid_: Glyphs, symbols
