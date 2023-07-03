using Meadow.Update;
using System.Collections;

namespace Meadow.Daemon;

public class UpdateCollection : IEnumerable<UpdateInfo>
{
    private List<UpdateDescriptor> _list;

    internal UpdateCollection()
    {
        _list = new List<UpdateDescriptor>();
    }

    public int Count => _list.Count;

    public UpdateInfo this[int index] => _list[index];
    public UpdateInfo this[string id] => _list.FirstOrDefault(d => string.Compare(d.ID, id, true) == 0);

    internal void Add(UpdateDescriptor updateDescriptor)
    {
        lock (_list)
        {
            var existing = _list.FirstOrDefault(d => string.Compare(d.ID, updateDescriptor.ID, true) == 0);
            if (existing == null)
            {
                _list.Add(updateDescriptor);
            }
        }
    }

    public IEnumerator<UpdateInfo> GetEnumerator()
    {
        return _list.GetEnumerator();
    }

    IEnumerator IEnumerable.GetEnumerator()
    {
        return GetEnumerator();
    }
}
