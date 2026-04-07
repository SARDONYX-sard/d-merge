import { createFileRoute } from '@tanstack/react-router';
import { Top } from '@/components/templates/Top';

export const Route = createFileRoute('/')({
  component: Top,
});
