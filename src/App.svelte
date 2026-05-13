<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface LiveSession {
    pid: number;
    session_id: string;
    cwd: string;
    status: string;
    name: string | null;
    version: string;
    kind: string;
  }

  interface Project {
    encoded_path: string;
    original_path: string;
    display_name: string;
    session_count: number;
  }

  let sessions = $state<LiveSession[]>([]);
  let projects = $state<Project[]>([]);

  async function refresh() {
    sessions = await invoke<LiveSession[]>("list_live_sessions");
    projects = await invoke<Project[]>("list_projects");
  }

  $effect(() => {
    refresh();
  });
</script>

<main>
  <h1>Ground Control</h1>

  <section>
    <h2>Live Sessions ({sessions.length})</h2>
    {#if sessions.length === 0}
      <p class="empty">No active sessions</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>PID</th>
            <th>Name</th>
            <th>Status</th>
            <th>Directory</th>
            <th>Version</th>
          </tr>
        </thead>
        <tbody>
          {#each sessions as s}
            <tr class={s.status}>
              <td class="mono">{s.pid}</td>
              <td>{s.name ?? "—"}</td>
              <td><span class="badge {s.status}">{s.status}</span></td>
              <td class="mono" title={s.cwd}>{s.cwd.split("/").pop()}</td>
              <td class="mono">{s.version}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <section>
    <h2>Projects ({projects.length})</h2>
    {#if projects.length === 0}
      <p class="empty">No projects found</p>
    {:else}
      <div class="grid">
        {#each projects as p}
          <div class="card">
            <h3>{p.display_name}</h3>
            <p class="mono path">{p.original_path}</p>
            <p class="meta">{p.session_count} session{p.session_count === 1 ? "" : "s"}</p>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <button onclick={refresh}>Refresh</button>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    background: #0a0a0a;
    color: #e0e0e0;
  }

  main {
    max-width: 960px;
    margin: 0 auto;
    padding: 2rem;
  }

  h1 {
    font-size: 1.5rem;
    font-weight: 600;
    margin-bottom: 2rem;
    color: #fff;
  }

  h2 {
    font-size: 1rem;
    font-weight: 500;
    color: #999;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.75rem;
  }

  section {
    margin-bottom: 2rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    color: #666;
    text-transform: uppercase;
    border-bottom: 1px solid #222;
  }

  td {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #1a1a1a;
  }

  .mono {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.85rem;
  }

  .badge {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge.busy {
    background: #1a3a1a;
    color: #4ade80;
  }

  .badge.idle {
    background: #1a1a2e;
    color: #818cf8;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.75rem;
  }

  .card {
    background: #141414;
    border: 1px solid #222;
    border-radius: 8px;
    padding: 1rem;
  }

  .card h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: #fff;
  }

  .path {
    font-size: 0.75rem;
    color: #666;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 0.8rem;
    color: #999;
    margin: 0.5rem 0 0;
  }

  .empty {
    color: #555;
    font-style: italic;
  }

  button {
    background: #222;
    color: #ccc;
    border: 1px solid #333;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  button:hover {
    background: #2a2a2a;
    color: #fff;
  }
</style>
