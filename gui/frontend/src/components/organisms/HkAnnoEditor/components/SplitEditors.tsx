import { Allotment } from 'allotment';
import { useEditorContext } from '../context/editorContext';
import { useMonacoSyncJump } from '../hooks/useMonacoSyncJump';
import { AnnotationEditor } from './AnnotationEditor';
import { EmptyDragPoint } from './EmptyDragPoint';
import { PreviewEditor } from './PreviewEditor';

import 'allotment/dist/style.css'; // NOTE: Please don't forget this line.

/** Annotation editor with preview pane */
export const SplitEditors = () => {
  const { registerLeft, registerRight, updateBaseLine } = useMonacoSyncJump();
  const [state, _] = useEditorContext();
  const tab = state.tabs[state.active];

  if (!tab) {
    return <EmptyDragPoint />;
  }

  return (
    <Allotment>
      <Allotment.Pane minSize={300}>
        <AnnotationEditor onMount={registerLeft} />
      </Allotment.Pane>

      {state.showPreview && (
        <Allotment.Pane minSize={200} preferredSize={650}>
          <PreviewEditor updateBaseLine={updateBaseLine} onMount={registerRight} />
        </Allotment.Pane>
      )}
    </Allotment>
  );
};
