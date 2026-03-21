import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/globe')({
  component: Globe,
})

function Globe() {
  return <div><h1>Globe</h1></div>
}
