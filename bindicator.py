def get_bins_for_date(events, date):

    (year, month, day, _, _, _, _, _) = date
    bins = set()

    for e in events:
        (e_year, e_month, e_day) = e['START']
        if year == e_year and month == e_month and day == e_day:
            bins.add(e['SUMMARY'])
    return bins
