import { createFileRoute } from '@tanstack/react-router';
import { Patch } from '@/components/templates/Patch';

export const Route = createFileRoute('/patch')({
  component: Patch,
});
