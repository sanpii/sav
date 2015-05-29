<?php

namespace Model;

use PommProject\ModelManager\Model\Model;
use PommProject\ModelManager\Model\Projection;
use PommProject\ModelManager\Model\ModelTrait\WriteQueries;

use PommProject\Foundation\Where;

use Model\AutoStructure\Expense as ExpenseStructure;
use Model\Expense;

/**
 * ExpenseModel
 *
 * Model class for table expense.
 *
 * @see Model
 */
class ExpenseModel extends Model
{
    use WriteQueries;

    /**
     * __construct()
     *
     * Model constructor
     *
     * @access public
     * @return void
     */
    public function __construct()
    {
        $this->structure = new ExpenseStructure;
        $this->flexible_entity_class = '\Model\Expense';
    }
}
