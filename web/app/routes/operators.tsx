import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/operators')({
  component: Operators,
})

function Operators() {
  return <div><h1>Operators</h1></div>
}
