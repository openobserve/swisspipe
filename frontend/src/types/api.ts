export interface ApiResponse<T> {
  data: T
  success: boolean
  message?: string
}

export interface ApiError {
  message: string
  status: number
  code?: string
}

export interface PaginationParams {
  page: number
  limit: number
  sort?: string
  order?: 'asc' | 'desc'
}

export interface FilterParams {
  search?: string
  status?: string[]
  created_after?: string
  created_before?: string
}

export interface WorkflowFilters extends FilterParams {
  // Additional workflow-specific filters
}

export interface TableSortConfig {
  column: string
  direction: 'asc' | 'desc'
}

export interface TableColumn {
  key: string
  label: string
  sortable: boolean
  width?: string
  align?: 'left' | 'center' | 'right'
}