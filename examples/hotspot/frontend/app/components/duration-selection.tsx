import { Clock } from 'lucide-react'

export function DurationSelector ({
  selected, onChange, disabled, durationOptions
}: {
  selected: number,
  onChange: (value: number) => void,
  disabled: boolean,
  durationOptions: { value: number, label: string, description: string }[]
}) {
  return (
  <div className="grid grid-cols-2 gap-4 pt-2">
    {durationOptions.map((option) => (
      <label
        key={option.value}
        className={`
          relative flex flex-col items-center justify-between rounded-lg border-2 p-4
          ${selected === option.value ? 'border-blue-500 bg-blue-50' : 'border-gray-200'}
          ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer hover:bg-gray-50'}
          transition-all duration-200
        `}
      >
        <input
          type="radio"
          name="duration"
          value={option.value}
          checked={selected === option.value}
          onChange={() => onChange(option.value)}
          disabled={disabled}
          className="sr-only"
        />
        <Clock className="h-6 w-6 mb-2 text-blue-600" />
        <div className="text-center">
          <h3 className="font-semibold text-gray-900">{option.label}</h3>
          <p className="text-sm text-gray-500">{option.description}</p>
        </div>
        {selected === option.value && (
          <div className="absolute top-2 right-2 w-3 h-3 rounded-full bg-blue-500" />
        )}
      </label>
    ))}
  </div>
  )
};