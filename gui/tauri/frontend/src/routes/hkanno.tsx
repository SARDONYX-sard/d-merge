import { createFileRoute } from '@tanstack/react-router';
import { HkannoEditorPage } from '@/components/templates/HkAnno';

export const Route = createFileRoute('/hkanno')({
  component: HkannoEditorPage,
});
