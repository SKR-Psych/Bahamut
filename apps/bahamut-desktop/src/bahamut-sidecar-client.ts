export class BahamutSidecarClient {
  private port: number = 0;
  private token: string = '';

  constructor() {
    // Read from environment or process globals injected by Electron
    if (typeof process !== 'undefined' && process.env) {
      this.port = parseInt(process.env.BAHAMUT_PORT || '0', 10);
      this.token = process.env.BAHAMUT_AUTH_TOKEN || '';
    }
  }

  public get isConfigured(): boolean {
    return this.port > 0 && this.token.length > 0;
  }

  public async pingOllama(): Promise<{ isRunning: boolean; models: string[] }> {
    if (!this.isConfigured) {
      throw new Error('Sidecar connection parameters not configured.');
    }

    const response = await fetch(`http://127.0.0.1:${this.port}/v1/ollama/status`, {
      method: 'GET',
      headers: {
        'X-Bahamut-Auth': this.token,
      },
    });

    if (response.status === 401) {
      throw new Error('Unauthorized sidecar access (invalid token).');
    }

    if (!response.ok) {
      throw new Error(`Failed to query Ollama status: ${response.statusText}`);
    }

    const data = await response.json();
    return {
      isRunning: data.is_running,
      models: data.installed_models,
    };
  }

  public async checkSandbox(): Promise<{ active: boolean; workspaceName?: string }> {
    if (!this.isConfigured) {
      throw new Error('Sidecar parameters not configured.');
    }

    const response = await fetch(`http://127.0.0.1:${this.port}/v1/sandbox`, {
      method: 'GET',
      headers: {
        'X-Bahamut-Auth': this.token,
      },
    });

    if (!response.ok) {
      throw new Error('Failed to query sandbox status.');
    }

    const data = await response.json();
    return {
      active: data.sandbox_active,
      workspaceName: data.workspace_name,
    };
  }
}
