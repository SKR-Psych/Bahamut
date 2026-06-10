import { ContainerModule } from '@theia/core/shared/inversify';
import { WidgetFactory, FrontendApplicationContribution, bindViewContribution } from '@theia/core/lib/browser';
import { BahamutAgentWidget } from './bahamut-widget';
import { BahamutSidecarClient } from './bahamut-sidecar-client';

export default new ContainerModule(bind => {
    // Bind Sidecar client
    bind(BahamutSidecarClient).toSelf().inSingletonScope();

    // Bind Agent Widget
    bind(BahamutAgentWidget).toSelf();
    bind(WidgetFactory).toDynamicValue(ctx => ({
        id: BahamutAgentWidget.ID,
        createWidget: () => ctx.container.get(BahamutAgentWidget)
    })).inSingletonScope();

    // Bind custom layout contribution to open panel on startup
    bind(FrontendApplicationContribution).toDynamicValue(ctx => ({
        onStart: (app) => {
            const widget = ctx.container.get(BahamutAgentWidget);
            app.shell.add(widget, 'right');
        }
    })).inSingletonScope();
});
