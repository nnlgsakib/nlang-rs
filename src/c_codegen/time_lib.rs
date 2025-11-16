use super::CCodeGenerator;

impl CCodeGenerator {
    pub fn emit_time_helpers(&mut self) {
        self.empty();
        self.line("// Time helpers");
        self.line("static long long nstd_timestamp(){ return (long long)time(NULL); }");
        self.line("static long long nstd_timestamp_ms(){ return (long long)time(NULL)*1000LL; }");
        self.line("static long long nstd_timestamp_us(){ return (long long)time(NULL)*1000000LL; }");
        self.line("static long long nstd_timestamp_ns(){ return (long long)time(NULL)*1000000000LL; }");
        self.line("static char* nstd_now_str(int utc){ time_t t=time(NULL); struct tm tmv; struct tm* p; if(utc){ p = gmtime(&t); if(p) tmv=*p; } else { p = localtime(&t); if(p) tmv=*p; } char* out=(char*)malloc(32); if(!out) return NULL; strftime(out, 31, \"%Y-%m-%d %H:%M:%S\", &tmv); out[31]=0; return out; }");
        self.line("static void nstd_sleep_ms(long long ms){");
        self.push();
        self.line("if(ms<=0) return;");
        self.line("#ifdef _WIN32");
        self.line("Sleep((DWORD)ms);");
        self.line("#else");
        self.line("struct timespec ts; ts.tv_sec = (time_t)(ms/1000); ts.tv_nsec = (long)((ms%1000)*1000000L);");
        self.line("nanosleep(&ts, NULL);");
        self.line("#endif");
        self.pop();
        self.line("}");
        self.line("static int nstd_year_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? (1900+p->tm_year): 0; }");
        self.line("static int nstd_month_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? (1+p->tm_mon): 0; }");
        self.line("static int nstd_day_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? p->tm_mday: 0; }");
        self.line("static int nstd_weekday_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? p->tm_wday: 0; }");
        self.line("static int nstd_hour_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? p->tm_hour: 0; }");
        self.line("static int nstd_minute_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? p->tm_min: 0; }");
        self.line("static int nstd_second_local(){ time_t t=time(NULL); struct tm* p=localtime(&t); return p? p->tm_sec: 0; }");
        self.line("static char* nstd_format_now_local(const char* fmt){ time_t t=time(NULL); struct tm* p=localtime(&t); char* out=(char*)malloc(128); if(!out) return NULL; strftime(out, 127, fmt, p); out[127]=0; return out; }");
        self.line("static char* nstd_to_local_from_secs(long long secs){ time_t t=(time_t)secs; struct tm* p=localtime(&t); char* out=(char*)malloc(32); if(!out) return NULL; strftime(out, 31, \"%Y-%m-%d %H:%M:%S\", p); out[31]=0; return out; }");
        self.line("static char* nstd_to_utc_from_secs(long long secs){ time_t t=(time_t)secs; struct tm* p=gmtime(&t); char* out=(char*)malloc(32); if(!out) return NULL; strftime(out, 31, \"%Y-%m-%d %H:%M:%S\", p); out[31]=0; return out; }");
        self.empty();
        self.line("// Parsing helpers");
        self.line("static long long nstd_parse_ymd(const char* s){ int y=0,m=0,d=0; if(sscanf(s, \"%d-%d-%d\", &y,&m,&d)!=3) return 0; struct tm t; memset(&t,0,sizeof(t)); t.tm_year=y-1900; t.tm_mon=m-1; t.tm_mday=d; t.tm_isdst=-1; return (long long)mktime(&t); }");
        self.line("#ifdef _WIN32\nstatic time_t nstd_timegm(struct tm* t){ return _mkgmtime(t); }\n#else\nstatic time_t nstd_timegm(struct tm* t){ return timegm(t); }\n#endif");
        self.line("static long long nstd_parse_rfc3339(const char* s){ int y=0,m=0,d=0,h=0,mi=0,sec=0; if(sscanf(s, \"%d-%d-%dT%d:%d:%dZ\", &y,&m,&d,&h,&mi,&sec)!=6) return 0; struct tm t; memset(&t,0,sizeof(t)); t.tm_year=y-1900; t.tm_mon=m-1; t.tm_mday=d; t.tm_hour=h; t.tm_min=mi; t.tm_sec=sec; return (long long)nstd_timegm(&t); }");
        self.line("static int nstd_mon_from_abbr(const char* s){ const char* months=\"JanFebMarAprMayJunJulAugSepOctNovDec\"; for(int i=0;i<12;i++){ if(strncmp(s, months+3*i, 3)==0) return i+1; } return 1; }");
        self.line("static long long nstd_parse_rfc2822(const char* s){ int d=0,y=0,h=0,mi=0,sec=0,tz=0; char mon[4]={0}; if(sscanf(s, \"%*[^,], %d %3s %d %d:%d:%d %d\", &d, mon, &y, &h, &mi, &sec, &tz)<6) return 0; int m = nstd_mon_from_abbr(mon); struct tm t; memset(&t,0,sizeof(t)); t.tm_year=y-1900; t.tm_mon=m-1; t.tm_mday=d; t.tm_hour=h; t.tm_min=mi; t.tm_sec=sec; // ignore tz for now
            return (long long)nstd_timegm(&t); }");
    }
}