interface ProjectUser {
  user_id: string;
  role: string;
}
export interface Project {
  task: {
    _id: string;
    group: {
      _id: string;
      name: string;
    };
    name: string;
    volume: {
      value: number;
      unit: string;
    };
    cost?: number;
    estimation?: {
      date: Date;
      value: number;
    }[];
    report_id?: string[];
    /*
     * Report Output
     * progress: {
     *  date: Date
     *  value: number
     * }
     */
  }[];
  user: ProjectUser[];
  name: string;
  value?: number;
  customer: {
    _id: string;
    person: {
      _id: string;
      name: string;
      role: string;
    };
  };
}

interface ProjectReport {
  _id: string;
  project_id: string;
  task: {
    _id: string;
    value: number;
    detail?: string[];
  }[];
  plan: {
    task_id: string;
    detail?: string[];
  }[];
  weather?: {
    time: [number, number];
    condition: "sunny" | "cloudy" | "rainy" | "heavy rain";
  }[];
  documentation?: {
    image_url: string;
    description?: string;
  }[];
  attendance_id: string;
  user_id: string;
  customer: {
    _id: string;
    person_id: string;
  };
  date: Date;
}

interface ProjectAttendance {
  _id: string;
  project_id: string;
  user: {
    _id: string;
    name: string;
    role: string;
    entry?: number;
    exit?: number;
    outsource?: {
      _id: string;
      name: string;
    };
  }[];
  date: Date;
}
