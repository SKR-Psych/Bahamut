import * as React from 'react';
import { ReactWidget } from '@theia/core/lib/browser';
import { injectable, postConstruct, inject } from '@theia/core/shared/inversify';
import { BahamutSidecarClient } from './bahamut-sidecar-client';

@injectable()
export class BahamutAgentWidget extends ReactWidget {
  static readonly ID = 'bahamut-agent-widget';
  static readonly LABEL = 'Bahamut Agent';

  protected readonly sidecarClient = new BahamutSidecarClient();

  @inject(BahamutSidecarClient)
  protected readonly injectedClient: BahamutSidecarClient;

  // React state for sidecar tests
  protected state: {
    mode: 'ide' | 'agent';
    ollamaStatus: string;
    sandboxStatus: string;
    loading: boolean;
  } = {
    mode: 'ide',
    ollamaStatus: 'Not verified',
    sandboxStatus: 'Not verified',
    loading: false,
  };

  @postConstruct()
  protected init(): void {
    this.id = BahamutAgentWidget.ID;
    this.title.label = BahamutAgentWidget.LABEL;
    this.title.caption = 'Bahamut Agent Controls';
    this.title.closable = false;
    this.addClass('bahamut-agent-panel');
    this.update();
  }

  protected async checkSidecar(): Promise<void> {
    this.state.loading = true;
    this.state.ollamaStatus = 'Checking...';
    this.state.sandboxStatus = 'Checking...';
    this.update();

    try {
      const ollama = await this.sidecarClient.pingOllama();
      this.state.ollamaStatus = ollama.isRunning 
        ? `Running (Models: ${ollama.models.join(', ') || 'None'})` 
        : 'Not running';
    } catch (e: any) {
      this.state.ollamaStatus = `Connection failed: ${e.message}`;
    }

    try {
      const sandbox = await this.sidecarClient.checkSandbox();
      this.state.sandboxStatus = sandbox.active 
        ? `Active (Workspace: ${sandbox.workspaceName})` 
        : 'Sandbox inactive';
    } catch (e: any) {
      this.state.sandboxStatus = `Query failed: ${e.message}`;
    }

    this.state.loading = false;
    this.update();
  }

  protected render(): React.ReactNode {
    return (
      <div>
        <h2 className="bahamut-title">Bahamut Agent Control Center</h2>
        <p style={{ color: '#a1a19a', fontSize: '0.9rem', marginBottom: '1.5rem' }}>
          Execute and monitor autonomous development runs.
        </p>

        {/* Mode switch bar */}
        <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '2rem' }}>
          <button 
            className="bahamut-primary-action" 
            style={{ opacity: this.state.mode === 'ide' ? 1 : 0.6, background: this.state.mode === 'ide' ? '#6F7448' : '#222' }}
            onClick={() => { this.state.mode = 'ide'; this.update(); }}
          >
            Bahamut IDE Mode
          </button>
          <button 
            className="bahamut-primary-action" 
            style={{ opacity: this.state.mode === 'agent' ? 1 : 0.6, background: this.state.mode === 'agent' ? '#6F7448' : '#222' }}
            onClick={() => { this.state.mode = 'agent'; this.update(); }}
          >
            Bahamut Agent Mode
          </button>
        </div>

        {/* Sidecar verification controls */}
        <div style={{ border: '1px solid rgba(255,255,255,0.06)', borderRadius: '8px', padding: '1rem', background: 'rgba(0,0,0,0.2)' }}>
          <h4 style={{ margin: '0 0 0.75rem 0' }}>Rust Sidecar Status Check</h4>
          <p style={{ fontSize: '0.8rem', color: '#888' }}>
            Tests authenticated communications over ephemeral port bindings.
          </p>

          <button 
            className="bahamut-primary-action" 
            onClick={() => this.checkSidecar()} 
            disabled={this.state.loading}
            style={{ width: '100%', marginBottom: '1rem' }}
          >
            {this.state.loading ? 'Verifying...' : 'Query Sidecar Daemon'}
          </button>

          <div style={{ fontSize: '0.85rem', lineHeight: '1.6' }}>
            <p><strong>Ollama Status:</strong> <code style={{ color: '#B98A84' }}>{this.state.ollamaStatus}</code></p>
            <p><strong>Sandbox Boundary:</strong> <code style={{ color: '#B98A84' }}>{this.state.sandboxStatus}</code></p>
          </div>
        </div>

        {/* Agent placeholder timeline */}
        <div style={{ marginTop: '2rem' }}>
          <h4 style={{ margin: '0 0 1rem 0' }}>Agent Operations Timeline</h4>
          
          <div className="bahamut-timeline-item">
            <div style={{ fontWeight: 'bold', fontSize: '0.85rem' }}>Agent Init</div>
            <div style={{ fontSize: '0.8rem', color: '#a1a19a' }}>Spike environment configured. Waiting for targets...</div>
          </div>
          
          <div className="bahamut-timeline-item">
            <div style={{ fontWeight: 'bold', fontSize: '0.85rem' }}>Platform Check</div>
            <div style={{ fontSize: '0.8rem', color: '#a1a19a' }}>Spike verified. Theia application shell running successfully.</div>
          </div>
        </div>
      </div>
    );
  }
}
